//! Essentially, `scrittore` is a `View` module (in the Model-View-Controller paradigm) which
//! is called by a Controller to render the collection of Notes.

use handlebars::{Handlebars, TemplateRenderError};
use serde_json::Value;
use std::collections::BTreeMap;

use super::{Durational, Note};
use super::sequenza::GroupingController;

pub struct View<'a, D: 'a + Durational> {
    pub source: String,
    pub data: BTreeMap<String, Value>,
    hb: Handlebars,
    controller: &'a mut GroupingController<D>
}

impl<'a, D: 'a + Durational> View<'a, D> {
    pub fn render(&self) -> Result<String, TemplateRenderError> {
        self.hb.template_render(&self.source, &self.data)
    }

    pub fn format_note<N: Note<D> + Clone>(&mut self, note: N) -> Result<String, &'static str> {
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
            out.push_str(format!("{}{}{} ~ ", note.text(), left.as_lilypond(), note.annotations()).as_str());

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

    pub fn format_notes<N: Note<D> + Clone>(&mut self, notes: Vec<N>) -> Result<String, &'static str> {
        Ok(notes.into_iter()
            .map(|n| self.format_note(n).unwrap())
            .collect::<Vec<String>>().join(" "))
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
    fn test_view() {
        let mut hb = Handlebars::new();
        let source = "{{#each notes as |note|}} {{{note}}} {{/each}}".to_string();
        let mut data = BTreeMap::new();
        let notes = vec![
            "c4".to_string(),
            "d4".to_string(),
            "e4".to_string()
        ];

        data.insert("notes".to_string(), handlebars::to_json(&notes));
        let exp = " c4  d4  e4 ".to_string();

        assert_eq!(hb.template_render(&source, &data).unwrap(), exp);

        let mut controller = initialize_controller();

        let view = View {
            hb: hb,
            source: source,
            data: data,
            controller: &mut controller
        };

        assert_eq!(view.render().unwrap(), exp);
    }

    #[test]
    fn test_format_note() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut hb = Handlebars::new();
        let source = "{{#each notes as |note|}} {{{ note.text }}} {{/each}}".to_string();
        let mut data = BTreeMap::new();
        data.insert("notes".to_string(), handlebars::to_json(&notes));
        let mut view = View {
            hb: hb,
            source: source,
            data: data,
            controller: &mut controller
        };

        assert_eq!(Ok(" %m. \n c4 ~ c4".to_string()), view.format_note(notes[0].clone()));
    }

    #[test]
    fn test_format_notes() {
        let notes = initialize_notes();
        let mut controller = initialize_controller();
        let mut hb = Handlebars::new();
        let source = "".to_string();
        let mut data = BTreeMap::new();
        data.insert("notes".to_string(), handlebars::to_json(&notes));
        let mut view = View {
            hb: hb,
            source: source,
            data: data,
            controller: &mut controller
        };

        assert_eq!(Ok(" %m. \n c4 ~ c4 d4 e4 |\n   %m. \n f4".to_string()), view.format_notes(notes));
    }
}

