//! Essentially, `scrittore` is a `View` module (in the Model-View-Controller paradigm) which
//! is called by a Controller to render the collection of Notes.

use handlebars::{Handlebars, Helper, RenderContext, TemplateError, RenderError};
use serde_json::{self, Value};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::error::Error;
use std::path::Path;

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
    pub context: BTreeMap<String, Value>,
    hb: Handlebars,
    phantom: PhantomData<(P, D)>
}

pub struct ChordView<P, D>
{
    pub context: BTreeMap<String, Value>,
    hb: Handlebars,
    phantom: PhantomData<(P, D)>
}

pub struct NotesView<N, D>
{
    pub context: BTreeMap<String, Value>,
    hb: Handlebars,
    phantom: PhantomData<(N, D)>
}

/// The fundamental trait for scrittore module. By convention, `format()` instantiates a global
/// variable as the expected name of the input. That is, a `SingleNoteView` will instantiate its Input
/// data as the JSON object `note`.
pub trait View: Sized
{
    type Input;

    fn new(source: Option<String>, context: BTreeMap<String, Value>) -> Result<Self, Box<Error>>;

    fn hb(&self) -> &Handlebars;
    fn context(&self) -> &BTreeMap<String, Value>;

    fn load_context(&mut self, _: &Self::Input) -> Result<(), &'static str> { Ok(()) }

    fn render<'b>(&'b mut self, input: &Self::Input) -> Result<String, &'static str> 
    {
        self.load_context(input);
        self.hb().render("template", &self.context()).map_err(|_| "Could not render")
    }

    fn default_template_path() -> &'static Path { Path::new("") }
    fn init_handlebars(source: Option<String>) -> Result<Handlebars, Box<Error>> 
    {
        let mut hb = Handlebars::new();
        // Override the default with a no-escape function
        let escape_fn = |s: &str| -> String { s.to_string() };
        hb.register_escape_fn(escape_fn);
        match source {
            Some(s) => hb.register_template_string("template", s)?,
            None => hb.register_template_file("template", Self::default_template_path())?
        }
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

    fn new(source: Option<String>, context: BTreeMap<String, Value>) -> Result<Self, Box<Error>> 
    {
        let hb: Handlebars = Self::init_handlebars(source)?;
        let phantom = PhantomData;
        Ok(SingleNoteView { context, hb, phantom })
    }

    fn hb(&self) -> &Handlebars { &self.hb }
    fn context(&self) -> &BTreeMap<String, Value> { &self.context }

    fn load_context(&mut self, input: &Self::Input) -> Result<(), &'static str> 
    {
        let in_val = serde_json::to_value(input).map_err(|e| "Could not parse note into value")?;
        self.context.insert("note".to_string(), in_val);
        Ok(())
    }

    fn default_template_path() -> &'static Path {
        Path::new("templates/single_note.hbs")
    }
}

impl<'a, P, D> View for ChordView<P, D>
where D: 'a + Durational + Serialize,
      P: Pitch + Clone + Serialize,
      for<'de> D: Deserialize<'de>
{
    type Input = Chord<P, D>;

    fn new(source: Option<String>, context: BTreeMap<String, Value>) -> Result<Self, Box<Error>> 
    {
        let hb: Handlebars = Self::init_handlebars(source)?;
        let phantom = PhantomData;
        Ok(ChordView { context, hb, phantom })
    }

    fn hb(&self) -> &Handlebars { &self.hb }
    fn context(&self) -> &BTreeMap<String, Value> { &self.context }

    fn load_context(&mut self, input: &Self::Input) -> Result<(), &'static str> {
        let in_val = serde_json::to_value(input).map_err(|e| "Could not parse chord into value")?;
        self.context.insert("chord".to_string(), in_val);
        Ok(())
    }

    fn default_template_path() -> &'static Path {
        Path::new("templates/chord.hbs")
    }
}

impl<'a, D, N> View for NotesView<N, D>
where D: 'a + Durational + Serialize,
      N: Note<D> + Clone + Serialize + Viewable<'a, D>,
      for<'de> D: Deserialize<'de>,
      for<'de> N: Deserialize<'de>
{
    type Input = Notes<N, D>;

    fn new(source: Option<String>, context: BTreeMap<String, Value>) -> Result<Self, Box<Error>> {
        let mut hb: Handlebars = Self::init_handlebars(source)?;
        hb.register_template_file("note", "templates/single_note.hbs")?;
        let view_note_helper = |h: &Helper, _: &Handlebars, rc: &mut RenderContext| -> Result<(), RenderError> {
            let viewable_json = h.param(0).map(|v| v.value())
                .ok_or(RenderError::new("Could not get param"))?;
            let note: N = serde_json::from_value(viewable_json.clone())
                .map_err(|e| RenderError::new(e.description()))?;
            let context: BTreeMap<String, Value> = serde_json::from_value(
                rc.context().data().clone())
                .map_err(|e| RenderError::new(e.description()))?;
            let mut view = <N as Viewable<D>>::View::new(None, context).
                map_err(|_| RenderError::new("Could not create view"))?;
            let out = note.render(&mut view)
                .map_err(|_| RenderError::new("Could not render"))?;
            rc.writer.write(out.trim().as_bytes().as_ref())?;
            Ok(())
        };
        hb.register_helper("view_note", Box::new(view_note_helper));
        let phantom = PhantomData;
        Ok(NotesView { context, hb, phantom })
    }

    fn hb(&self) -> &Handlebars { &self.hb }
    fn context(&self) -> &BTreeMap<String, Value> { &self.context }

    fn load_context(&mut self, input: &Self::Input) -> Result<(), &'static str> {
        let in_val = serde_json::to_value(&input.data).map_err(|e| "Could not parse notes into value")?;
        self.context.insert("notes".to_string(), in_val);
        Ok(())
    }

    fn default_template_path() -> &'static Path {
        &Path::new("templates/notes.hbs")
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
      N: Note<D> + Clone + Serialize + Viewable<'a, D>,
      for<'de> D: Deserialize<'de>,
      for<'de> N: Deserialize<'de>
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
    fn test_render_note_custom_template() {
        let notes = initialize_notes();
        let context = BTreeMap::new();
        let mut view = SingleNoteView::new(
            Some("{{ note.text }}".to_string()),
            context).unwrap();

        let out = notes[0].render(&mut view).unwrap();
        assert_eq!("c", &out);
    }

    #[test]
    fn test_render_note_template() {
        let notes = initialize_notes();
        let mut context = BTreeMap::new();
        let mut view = SingleNoteView::new(None, context).unwrap();
        let out = notes[0].render(&mut view).unwrap();
        assert_eq!("c2\n", out);
    }

    #[test]
    fn test_render_notes_template() {
        let notes = Notes::new(initialize_notes());
        let mut context = BTreeMap::new();
        let mut view = NotesView::new(None, context).unwrap();
        let out = notes.render(&mut view).unwrap();
        assert_eq!(" c2  d4  e4  f4 \n", out);
    }

    #[test]
    fn test_render_chord_template() {
        let chord: Chord<ETPitch, RatioDuration> = Chord::new(vec![ETPitch::new(60), ETPitch::new(62)], Duration(RatioDuration(1, 2)));
        let mut view = ChordView::new(None, BTreeMap::new()).unwrap();
        let out = chord.render(&mut view).unwrap();
        assert_eq!("< c  d >2\n", &out);
    }
}

