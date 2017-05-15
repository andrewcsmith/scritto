//! Essentially, `scrittore` is a `View` module (in the Model-View-Controller paradigm) which
//! is called by a Controller to render the collection of Notes.

use handlebars::{Handlebars, Helper, RenderContext, TemplateError, RenderError};
use serde_json::{self, Value};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::error::Error;

use super::{Pitch, Duration, Durational, IntegerDuration, RatioDuration, Note};
use super::sequenza::GroupingController;
use super::notes::{ETPitch, SingleNote, Chord};

#[derive(Clone, Serialize, Deserialize)]
pub struct Notes<N, D>
where N: Note<D>,
      D: Durational
{
    data: Vec<N>,
    phantom: PhantomData<D>
}

pub struct NoteView
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars
}

pub struct ChordView
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars
}

pub struct NotesView
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars
}

/// The fundamental trait for scrittore module. By convention, `format()` instantiates a global
/// variable as the expected name of the input. That is, a `NoteView` will instantiate its Input
/// data as the JSON object `note`.
pub trait View<D: Durational, Input> 
{
    fn format<'b>(&'b mut self, input: &Input) -> Result<String, &'static str>;
}

/// `Viewable` sets up a given context allowing for a single element to be rendered. An object will
/// receive a given `View`, and by convention insert itself into the data structure of that `View`
/// before rendering.
pub trait Viewable<'a, D>: Sized 
where D: 'a + Durational
{
    type View: View<D, Self>;

    fn render<'b>(&self, view: &'b mut Self::View) -> Result<String, &'static str> 
    {
        view.format(self)
    }
}

impl<N, D> Notes<N, D> 
where N: Note<D>,
      D: Durational
{
    pub fn new(notes: Vec<N>) -> Self {
        Notes {
            data: notes,
            phantom: PhantomData
        }
    }
}

impl NoteView
{
    pub fn new(source: String, data: BTreeMap<String, Value>) -> Result<Self, TemplateError> 
    {
        let view = NoteView {
            data: data,
            hb: Self::init_handlebars(source)?
        };

        Ok(view)
    }

    fn init_handlebars(source: String) -> Result<Handlebars, TemplateError> 
    {
        let mut hb = Handlebars::new();
        // Override the default with a no-escape function
        let escape_fn = |s: &str| -> String { s.to_string() };
        hb.register_escape_fn(escape_fn);
        hb.register_template_string("template", source)?;
        Ok(hb)
    }

    pub fn render(&self) -> Result<String, RenderError> {
        self.hb.render("template", &self.data)
    }
}

impl NotesView
{
    pub fn new(source: String, data: BTreeMap<String, Value>) -> Result<Self, TemplateError> {
        let mut view = NotesView {
            data: data,
            hb: Self::init_handlebars(source)?
        };

        // let format_note_helper = |h: &Helper, _: &Handlebars, rc: &mut RenderContext| -> Result<(), RenderError> {
        //     match h.param(0).ok_or(RenderError::new("format_note expects 1 param"))?.value() {
        //         &Value::Object(ref s) => {
        //             let note: Box<Note<D>> = match s.get("pitch_type").unwrap() {
        //                 &Value::String(ref x) if x.as_str() == "ETPitch" => {
        //                     let note: SingleNote<ETPitch, D> = serde_json::from_value(Value::Object(s.clone()))
        //                         .map_err(|e| RenderError::new(e.description()))?;
        //                     Box::new(note)
        //                 },
        //                 _ => { return Err(RenderError::new("Not a known Pitch type")) }
        //             };
        //
        //             let out = format!("{}{}", note.text(), note.duration().as_lilypond());
        //             rc.writer.write(out.into_bytes().as_ref())?;
        //         },
        //         _ => { }
        //     }
        //
        //     Ok(())
        // };
        //
        // let ly_helper = |h: &Helper, _: &Handlebars, rc: &mut RenderContext| -> Result<(), RenderError> {
        //     match h.param(0).unwrap().value() {
        //         &Value::Object(ref s) => {
        //             let dur_value = s.get("duration").ok_or(RenderError::new("Param 0 should implement Durational"))?;
        //             let duration: Duration<RatioDuration> = serde_json::from_value(dur_value.clone()).unwrap();
        //             let out = format!("{}", duration.as_lilypond());
        //             rc.writer.write(out.into_bytes().as_ref())?;
        //         },
        //         &Value::Array(ref val) => {
        //             let duration: Duration<D> = serde_json::from_value(Value::Array(val.clone()))
        //                 .map_err(|e| RenderError::new(e.description()))?;
        //             let out = format!("{}", duration.as_lilypond());
        //             rc.writer.write(out.as_bytes().as_ref())?;
        //         },
        //         _ => { }
        //     }
        //     Ok(())
        // };
        //
        // view.hb.register_helper("format_note", Box::new(format_note_helper));
        // view.hb.register_helper("ly", Box::new(ly_helper));

        Ok(view)
    }

    /// Overrides the default Handlebars escape function with a no-op. In the future, perhaps this
    /// should escape for Lilypond.
    fn init_handlebars(source: String) -> Result<Handlebars, TemplateError> {
        let mut hb = Handlebars::new();
        // Override the default with a no-escape function
        let escape_fn = |s: &str| -> String { s.to_string() };
        hb.register_escape_fn(escape_fn);
        hb.register_template_string("template", source)?;
        Ok(hb)
    }

    pub fn render(&self) -> Result<String, RenderError> {
        self.hb.render("template", &self.data)
    }
}

impl<'a, D, N> View<D, N> for NoteView
where D: 'a + Durational,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    fn format<'b>(&'b mut self, input: &N) -> Result<String, &'static str> {
        let in_val = serde_json::to_value(input).map_err(|e| "Could not parse note into value")?;
        self.data.insert("note".to_string(), in_val);
        self.render().map_err(|_| "Could not render")
    }
}

// impl<'a, D, P> View<D, Chord<P, D>> for ChordView
// where D: 'a + Durational + Serialize,
//       P: Pitch + Clone + Serialize,
//       for<'de> D: Deserialize<'de>
// {
//     fn format<'b>(&'b mut self, input: &Chord<P, D>) -> Result<String, &'static str> {
//         let in_val = serde_json::to_value(input).map_err(|e| "Could not parse chord into value")?;
//         self.data.insert("chord".to_string(), in_val);
//         self.render().map_err(|_| "Could not render")
//     }
// }

impl<'a, D, N> View<D, Notes<N, D>> for NotesView
where D: 'a + Durational,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    fn format<'b>(&'b mut self, input: &Notes<N, D>) -> Result<String, &'static str> {
        let in_val = serde_json::to_value(&input.data).map_err(|e| "Could not parse notes into value")?;
        self.data.insert("notes".to_string(), in_val);
        self.render().map_err(|_| "Could not render")
    }
}

impl<'a, P, D> Viewable<'a, D> for SingleNote<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = NoteView;
}

impl<'a, P, D> Viewable<'a, D> for Chord<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = NoteView;
}

impl<'a, D, N> Viewable<'a, D> for Notes<N, D>
where D: 'a + Durational + Serialize,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = NotesView;
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::*;
    use super::super::notes::*;
    use super::super::sequenza::{Measure, Beat};
    
    fn initialize_notes() -> Vec<SingleNote<ETPitch, RatioDuration>> {
        vec![
            SingleNote::new(ETPitch::new(60), Duration(RatioDuration(1, 2))),
            SingleNote::new(ETPitch::new(62), Duration(RatioDuration(1, 4))),
            SingleNote::new(ETPitch::new(64), Duration(RatioDuration(1, 4))),
            SingleNote::new(ETPitch::new(65), Duration(RatioDuration(1, 4)))
        ]
    }

    #[test]
    fn test_render_note() {
        let notes = initialize_notes();
        let mut data = BTreeMap::new();
        let mut view = NoteView::new(
            "{{ note.text }}{{ note.ly_duration}}".to_string(),
            data).unwrap();

        let out = notes[0].render(&mut view).unwrap();
        assert_eq!("c2", &out);
    }

    #[test]
    fn test_render_chord() {
        let chord: Chord<ETPitch, RatioDuration> = Chord::new(vec![ETPitch::new(60), ETPitch::new(62)], Duration(RatioDuration(1, 2)));
        let mut data = BTreeMap::new();
        let mut view = NoteView::new(
            "<{{#each note.pitches as |pitch| }} {{ pitch.ly }} {{ /each }}>{{ note.ly_duration}}".to_string(),
            data).unwrap();

        let out = chord.render(&mut view).unwrap();
        assert_eq!("< c  d >2", &out);
    }

    #[test]
    fn test_render_notes() {
        let notes = Notes::new(initialize_notes());
        let mut data = BTreeMap::new();
        let mut view = NotesView::new(
            "{{ #each notes }} {{ text }}{{ ly_duration }} {{ /each }}".to_string(),
            data).unwrap();

        let out = notes.render(&mut view).unwrap();
        assert_eq!(" c2  d4  e4  f4 ", out);
    }
}

