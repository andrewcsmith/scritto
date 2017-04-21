# Scritto

In short, this project aims to provide an engine for transcribing data into
[Lilypond](http://www.lilypond.org)-notated music. Certain design and pipeline
decisions are heavily informed by Ruby on Rails, but with the added assurance
of proper trait and type resolution that comes with Rust.

## Suggestions

I highly recommend using `lyp` with this project. I anticipate using the server
capacity of `lyp` to render Lilypond scores in the background.

I have also worked to keep the data-processing sides of this application
distinct from the Lilypond sides. In other words, it should be possible to
translate the bulk of the specific generation methods to a format like
MusicXML, although I haven't tried to do so.

Also, you may notice there's basically no code here.

