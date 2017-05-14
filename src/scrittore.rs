//! Essentially, `scrittore` is a `View` module (in the Model-View-Controller paradigm) which
//! is called by a Controller to render the collection of Notes.

use handlebars::{Handlebars, Helper, RenderContext, TemplateError, RenderError};
use serde_json::{self, Value};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::error::Error;

use super::{Duration, Durational, IntegerDuration, RatioDuration, Note};
use super::sequenza::GroupingController;
use super::notes::{ETPitch, SingleNote};

pub trait View<D: Durational, Input> {
    fn format<'b>(&'b mut self, input: Input) -> Result<String, &'static str>;
}

pub trait Viewable<'a, D>: Sized 
where D: 'a + Durational
{
    type View: View<D, Self>;

    fn format<'b>(self, view: &'b mut Self::View) -> Result<String, &'static str>;
}

pub struct NotesView<'a, D> 
where D: 'a + Durational
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars,
    controller: &'a mut GroupingController<D>
}

impl<'a, D> NotesView<'a, D> 
where D: 'a + Durational,
      for<'de> D: Deserialize<'de>
{
    pub fn new(source: String, data: BTreeMap<String, Value>, controller: &'a mut GroupingController<D>) -> Result<Self, TemplateError> {
        let mut view = NotesView {
            data: data,
            hb: Self::init_handlebars(source)?,
            controller: controller
        };

        let format_note_helper = |h: &Helper, _: &Handlebars, rc: &mut RenderContext| -> Result<(), RenderError> {
            match h.param(0).ok_or(RenderError::new("format_note expects 1 param"))?.value() {
                &Value::Object(ref s) => {
                    let note: Box<Note<D>> = match s.get("pitch_type").unwrap() {
                        &Value::String(ref x) if x.as_str() == "ETPitch" => {
                            let note: SingleNote<ETPitch, D> = serde_json::from_value(Value::Object(s.clone()))
                                .map_err(|e| RenderError::new(e.description()))?;
                            Box::new(note)
                        },
                        _ => { return Err(RenderError::new("Not a known Pitch type")) }
                    };

                    let out = format!("{}{}", note.text(), note.duration().as_lilypond());
                    rc.writer.write(out.into_bytes().as_ref())?;
                },
                _ => { }
            }

            Ok(())
        };

        let ly_helper = |h: &Helper, _: &Handlebars, rc: &mut RenderContext| -> Result<(), RenderError> {
            match h.param(0).unwrap().value() {
                &Value::Object(ref s) => {
                    let dur_value = s.get("duration").ok_or(RenderError::new("Param 0 should implement Durational"))?;
                    let duration: Duration<RatioDuration> = serde_json::from_value(dur_value.clone()).unwrap();
                    let out = format!("{}", duration.as_lilypond());
                    rc.writer.write(out.into_bytes().as_ref())?;
                },
                &Value::Array(ref val) => {
                    let duration: Duration<D> = serde_json::from_value(Value::Array(val.clone()))
                        .map_err(|e| RenderError::new(e.description()))?;
                    let out = format!("{}", duration.as_lilypond());
                    rc.writer.write(out.as_bytes().as_ref())?;
                },
                _ => { }
            }
            Ok(())
        };

        view.hb.register_helper("format_note", Box::new(format_note_helper));
        view.hb.register_helper("ly", Box::new(ly_helper));

        Ok(view)
    }

    fn init_handlebars(source: String) -> Result<Handlebars, TemplateError> {
        let mut hb = Handlebars::new();
        // Override the default with a no-escape function
        let escape_fn = |s: &str| -> String { s.to_string() };
        hb.register_escape_fn(escape_fn);
        hb.register_template_string("start_group", "{{ note.text }}{{ note.ly_duration }}{{ note.annotations }} ~ ")?;
        hb.register_template_string("template", source)?;
        Ok(hb)
    }

    pub fn render(&self) -> Result<String, RenderError> {
        self.hb.render("template", &self.data)
    }

    pub fn format_note<N>(&mut self, note: N) -> Result<String, &'static str> 
        where N: Note<D> + Clone + Serialize
    {
        // Since Durational: Copy, this creates a temporary copy
        let mut dur = note.duration();
        let mut out = String::new();

        for g in self.controller.stack.iter() {
            if g.is_start_of_grouping() {
                out.push_str(g.grouping.start_annotation());
            }
        }

        // If the note overflows the current grouping...
        if self.controller.current()?.left < dur {
            let left = self.controller.current()?.left;
            // out.push_str(format!("{}{}{} ~ ", note.text(), left.as_lilypond(), note.annotations()).as_str());
            let mut map = BTreeMap::new();
            let mut tmp_note = note.clone();
            tmp_note.set_duration(left);

            map.insert("note", serde_json::to_value(tmp_note).unwrap());
            out.push_str(self.hb.render("start_group", &map).map_err(|_| "Could not start group")?.as_str());

            for g in self.controller.consume_time(left)? {
                out.push_str(g.end_annotation());
            }

            dur = dur - left;

            while self.controller.current()?.left <= dur {
                let left = self.controller.current()?.left;
                out.push_str(format!("{}{}", note.text(), left.as_lilypond()).as_str());
                dur = dur - left;
                if dur.as_float() > 0.0 {
                    out.push_str(" ~ ");
                }

                for g in self.controller.consume_time(left)? {
                    out.push_str(g.end_annotation());
                }
            }

            if dur.as_float() > 0.0 {
                out.push_str(format!("{}{}", note.text(), self.controller.current()?.left.as_lilypond()).as_str());
            }

            Ok(out)
        } else {
            out.push_str(format!("{}{}{}", note.text(), dur.as_lilypond(), note.annotations()).as_str());
            for g in self.controller.consume_time(dur)? {
                out.push_str(g.end_annotation());
            }
            Ok(out)
        }
    }

    pub fn format_notes<N>(&mut self, notes: Vec<N>) -> Result<String, &'static str> 
        where N: Note<D> + Clone + Serialize
    {
        Ok(notes.into_iter()
            .map(|n| self.format_note(n).unwrap_or(String::new()))
            .collect::<Vec<String>>().join(" "))
    }
}

impl<'a, D, N> View<D, Vec<N>> for NotesView<'a, D> 
where D: 'a + Durational,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    fn format<'b>(&'b mut self, input: Vec<N>) -> Result<String, &'static str> {
        self.format_notes(input)
    }
}

impl<'a, D, N> Viewable<'a, D> for Vec<N>
where D: 'a + Durational,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = NotesView<'a, D>;

    fn format<'b>(self, view: &'b mut Self::View) -> Result<String, &'static str> {
        view.format(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use super::super::notes::*;
    use super::super::sequenza::{Measure, Beat};
    
    fn initialize_notes() -> Vec<SingleNote<ETPitch, RatioDuration>> {
        vec![
            SingleNote::new(ETPitch(60), Duration(RatioDuration(1, 2))),
            SingleNote::new(ETPitch(62), Duration(RatioDuration(1, 4))),
            SingleNote::new(ETPitch(64), Duration(RatioDuration(1, 4))),
            SingleNote::new(ETPitch(65), Duration(RatioDuration(1, 4)))
        ]
    }

    // Generates two bars of 4/4
    fn initialize_groupings() -> Vec<Box<Grouping<RatioDuration>>> {
        vec![
            Box::new(Measure::from_contents(
                vec![
                    Box::new(Beat::new_ratio(1, 4)),
                    Box::new(Beat::new_ratio(1, 4)),
                    Box::new(Beat::new_ratio(1, 4)),
                    Box::new(Beat::new_ratio(1, 4))
                ]
            )),
            Box::new(Measure::from_contents(
                vec![
                    Box::new(Beat::new_ratio(1, 4)),
                    Box::new(Beat::new_ratio(1, 4)),
                    Box::new(Beat::new_ratio(1, 4)),
                    Box::new(Beat::new_ratio(1, 4))
                ]
            ))
        ]
    }

    fn initialize_controller() -> GroupingController<RatioDuration> {
        let groupings = initialize_groupings();
        GroupingController::new(Box::new(groupings.into_iter())).unwrap()
    }

    #[test]
    fn test_format_note() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut view = NotesView::new(
            String::new(),
            BTreeMap::new(),
            &mut controller).unwrap();

        assert_eq!(Ok(" %m. \n c4 ~ c4".to_string()), view.format_note(notes[0].clone()));
    }

    #[test]
    fn test_format_notes() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut view = NotesView::new(
            String::new(),
            BTreeMap::new(),
            &mut controller).unwrap();

        assert_eq!(Ok(" %m. \n c4 ~ c4 d4 e4 |\n   %m. \n f4".to_string()), view.format_notes(notes));
    }

    #[test]
    fn test_format() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut view = NotesView::new(
            String::new(),
            BTreeMap::new(),
            &mut controller).unwrap();

        assert_eq!(Ok(" %m. \n c4 ~ c4 d4 e4 |\n   %m. \n f4".to_string()), view.format(notes));
    }

    #[test]
    fn test_viewable_format() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut view = NotesView::new(
            "{{{ notes }}}".to_string(),
            BTreeMap::new(),
            &mut controller).unwrap();

        // let ref mut v = view;
        let out0 = vec![notes[0].clone()].format(&mut view).unwrap();
        let out1 = vec![notes[1].clone()].format(&mut view).unwrap();

        assert_eq!(" %m. \n c4 ~ c4".to_string(), out0);
        assert_eq!("d4".to_string(), out1);
    }

    #[test]
    fn test_render_with_template() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut data = BTreeMap::new();
        data.insert("note".to_string(), serde_json::to_value(&notes[0].clone()).unwrap());
        let view = NotesView::new(
            "{{ note.text }}{{ ly note.duration }}".to_string(),
            data,
            &mut controller).unwrap();

        view.render().unwrap();
        assert_eq!("c2".to_string(), view.render().unwrap());
    }

    #[test]
    fn test_render_with_helpers() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut data = BTreeMap::new();
        let mut templates = BTreeMap::new();
        templates.insert("start_group", "{{ note.text }}{{ note.duration.as_lilypond }}{{ note.start_annotation }}{{ tie }}");
        templates.insert("continue_group", "{{ note.text }}{{ note.duration.as_lilypond }}{{ tie }}");
        templates.insert("end_group", "{{ note.text }}{{ note.duration.as_lilypond }}");
        templates.insert("solo_note", "{{ note.text }}{{ note.duration.as_lilypond }}{{ note.start_annotation }}");

        let mut view = NotesView::new(
            "{{ formatted_notes }}".to_string(),
            data,
            &mut controller).unwrap();

        view.data.insert("note".to_string(), serde_json::to_value(&notes[0].clone()).unwrap());
        view.data.insert("tie".to_string(), serde_json::to_value(" ~ ").unwrap());

        let formatted_notes: String = view.format_note(notes[0].clone()).unwrap();
        view.data.insert("formatted_notes".to_string(), serde_json::to_value(formatted_notes.clone()).unwrap());

        view.render().unwrap();
        assert_eq!(" %m. \n c4 ~ c4".to_string(), view.render().expect("Panic on view.render()"));
    }
}

