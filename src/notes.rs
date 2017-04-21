//! The core traits for pitch and duration generalization in `scritto`. All data representing a
//! point in time that passes through the program will need to implement `Note` in some form, while
//! `Pitch` is specific to translating the onset of the `Note` into text.

trait Note {
    /// Duration of the `Note` should be given as a ratio tuple. This is to facilitate working with
    /// metrical divisions, including potential tuplets.
    fn duration(&self) -> (u32, u32);

    /// Text of the beginning of the note (excluding duration), which will be passed on to the
    /// given template. This appears at the start of the Note and is repeated as necessary
    /// following any ties.
    fn text(&self) -> String;

    /// Annotation text that will be printed above the initial note onset, but not at any later
    /// points.
    fn annotations(&self) -> &str {
        &""
    }
}

/// Responsible in many of the in-house stock cases for translating the onset of the `Note` into
/// text. This includes 12-tone equal tempered pitches (which are provided) as well as rational
/// pitches that take the form of the Helmholtz-Ellis accidentals as written in the Lilypond HE
/// library created by Andrew C. Smith.
trait Pitch {
    /// The only required method is one which translates the starting pitch to a note name of some
    /// sort, needed for the start of each `Note`.
    fn pitch(&self) -> String;
}

/// On the incomprehensible reason you would want to use equal temperament, this quicky is provided
/// to translate midi note values into easy chord names.
struct ETPitch(u32);

static ET_SCALE: [&str; 12] = ["c", "csharp", "d", "eflat", "e", "f", "fsharp", "g", "gsharp", "a", "bflat", "b"];

impl ETPitch {
    fn new(midi_value: u32) -> Self {
        ETPitch(midi_value)
    }
}

impl Pitch for ETPitch {
    fn pitch(&self) -> String {
        ET_SCALE[self.0 as usize % 12].to_string()
    }
}

impl From<u32> for ETPitch {
    fn from(f: u32) -> ETPitch {
        ETPitch(f)
    }
}

struct SingleNote<P: Pitch> {
    duration: Option<(u32, u32)>,
    pitch: P 
}

impl<P: Pitch> SingleNote<P> {
    fn new<IntoP: Into<P>, T: Into<Option<(u32, u32)>>>(pitch: IntoP, duration: T) -> Self {
        Self {
            duration: duration.into(),
            pitch: pitch.into()
        }
    }
}

impl<P: Pitch> Note for SingleNote<P> {
    fn duration(&self) -> (u32, u32) {
        self.duration.unwrap_or((1, 1))
    }

    fn text(&self) -> String {
        self.pitch.pitch()
    }
}

struct Chord<P: Pitch> {
    duration: Option<(u32, u32)>,
    pitches: Vec<P>
}

impl<P: Pitch> Chord<P> {
    fn new<T: Into<Option<(u32, u32)>>>(pitches: Vec<P>, duration: T) -> Self {
        Self {
            duration: duration.into(),
            pitches: pitches
        }
    }
}

impl<P: Pitch> Note for Chord<P> {
    fn duration(&self) -> (u32, u32) {
        self.duration.unwrap_or((1, 1))
    }

    fn text(&self) -> String {
        assert!(self.pitches.len() > 0);
        let mut out = String::with_capacity(self.pitches.len() * 2 + 1);
        out.push('<');
        // Push all but the last character
        for pitch in self.pitches[0..self.pitches.len()-1].iter() {
            out.push_str(&pitch.pitch()[..]);
            out.push(' ');
        }
        // Push the last character
        out.push_str(&self.pitches[self.pitches.len()-1].pitch()[..]);
        out.push('>');
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translates_midi_to_note_name() {
        assert_eq!(ETPitch(60).pitch(), "c");
        assert_eq!(ETPitch(69).pitch(), "a");
    }

    #[test]
    fn gets_single_note_name() {
        let note = SingleNote::<ETPitch>::new(ETPitch(62), None);
        assert_eq!(note.text().as_str(), "d");
    }

    #[test]
    fn gets_chord_name() {
        let chord = Chord::<ETPitch>::new(vec![ETPitch(60), ETPitch(64), ETPitch(67)], None);
        assert_eq!(chord.text().as_str(), "<c e g>");
    }

    #[test]
    fn one_note_chord() {
        let chord = Chord::new(vec![ETPitch(60)], None);
        assert_eq!(chord.text().as_str(), "<c>");
    }

    #[test]
    #[should_panic]
    fn panic_on_empty_chord() {
        let chord = Chord::<ETPitch>::new(vec![], None);
        chord.text().as_str();
    }
}

