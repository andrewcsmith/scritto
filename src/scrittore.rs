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

pub struct SingleNoteView<P, D>
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars,
    phantom: PhantomData<(P, D)>
}

pub struct ChordView<P, D>
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars,
    phantom: PhantomData<(P, D)>
}

pub struct NotesView<N, D>
{
    pub data: BTreeMap<String, Value>,
    hb: Handlebars,
    phantom: PhantomData<(N, D)>
}

/// The fundamental trait for scrittore module. By convention, `format()` instantiates a global
/// variable as the expected name of the input. That is, a `SingleNoteView` will instantiate its Input
/// data as the JSON object `note`.
pub trait View: Sized
{
    type Input;

    fn new(source: String, data: BTreeMap<String, Value>) -> Result<Self, TemplateError>;

    fn render<'b>(&'b mut self, input: &Self::Input) -> Result<String, &'static str>;

    fn init_handlebars(source: String) -> Result<Handlebars, TemplateError> 
    {
        let mut hb = Handlebars::new();
        // Override the default with a no-escape function
        let escape_fn = |s: &str| -> String { s.to_string() };
        hb.register_escape_fn(escape_fn);
        hb.register_template_string("template", source)?;
        Ok(hb)
    }
}

/// `Viewable` sets up a given context allowing for a single element to be rendered. An object will
/// receive a given `View`, and by convention insert itself into the data structure of that `View`
/// before rendering.
pub trait Viewable<'a, D>: Sized 
where D: 'a + Durational
{
    type View: View<Input=Self>;

    fn render<'b>(&self, view: &'b mut Self::View) -> Result<String, &'static str> 
    {
        view.render(self)
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

impl<'a, P, D> View for SingleNoteView<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type Input = SingleNote<P, D>;

    fn new(source: String, data: BTreeMap<String, Value>) -> Result<Self, TemplateError> 
    {
        let hb: Handlebars = Self::init_handlebars(source)?;
        let phantom = PhantomData;
        Ok(SingleNoteView { data, hb, phantom })
    }

    fn render<'b>(&'b mut self, input: &Self::Input) -> Result<String, &'static str> 
    {
        let in_val = serde_json::to_value(input).map_err(|e| "Could not parse note into value")?;
        println!("{}", serde_json::to_string(&in_val).unwrap());
        self.data.insert("note".to_string(), in_val);
        self.hb.render("template", &self.data).map_err(|_| "Could not render")
    }
}

impl<'a, P, D> View for ChordView<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type Input = Chord<P, D>;

    fn new(source: String, data: BTreeMap<String, Value>) -> Result<Self, TemplateError> 
    {
        let hb: Handlebars = Self::init_handlebars(source)?;
        let phantom = PhantomData;
        Ok(ChordView { data, hb, phantom })
    }

    fn render<'b>(&'b mut self, input: &Self::Input) -> Result<String, &'static str> {
        let in_val = serde_json::to_value(input).map_err(|e| "Could not parse chord into value")?;
        println!("{}", serde_json::to_string(&in_val).unwrap());
        self.data.insert("chord".to_string(), in_val);
        self.hb.render("template", &self.data).map_err(|_| "Could not render")
    }
}

impl<'a, D, N> View for NotesView<N, D>
where D: 'a + Durational + Serialize,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type Input = Notes<N, D>;

    fn new(source: String, data: BTreeMap<String, Value>) -> Result<Self, TemplateError> {
        let hb: Handlebars = Self::init_handlebars(source)?;
        let phantom = PhantomData;
        Ok(NotesView { data, hb, phantom })
    }

    fn render<'b>(&'b mut self, input: &Notes<N, D>) -> Result<String, &'static str> {
        let in_val = serde_json::to_value(&input.data).map_err(|e| "Could not parse notes into value")?;
        println!("{}", serde_json::to_string(&in_val).unwrap());
        self.data.insert("notes".to_string(), in_val);
        self.hb.render("template", &self.data).map_err(|_| "Could not render")
    }
}

impl<'a, P, D> Viewable<'a, D> for SingleNote<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = SingleNoteView<P, D>;
}

impl<'a, P, D> Viewable<'a, D> for Chord<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = ChordView<P, D>;
}

impl<'a, D, N> Viewable<'a, D> for Notes<N, D>
where D: 'a + Durational + Serialize,
      N: Note<D> + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type View = NotesView<N, D>;
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
        let mut view = SingleNoteView::new(
            "{{ note.text }}{{ note.ly_duration}}".to_string(),
            data).unwrap();

        let out = notes[0].render(&mut view).unwrap();
        assert_eq!("c2", &out);
    }

    #[test]
    fn test_render_chord() {
        let chord: Chord<ETPitch, RatioDuration> = Chord::new(vec![ETPitch::new(60), ETPitch::new(62)], Duration(RatioDuration(1, 2)));
        let mut data = BTreeMap::new();
        let mut view = ChordView::new(
            "<{{#each chord.pitches as |pitch| }} {{ pitch.ly }} {{ /each }}>{{ chord.ly_duration}}".to_string(),
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

