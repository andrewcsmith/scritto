# Scritto

In short, this project aims to provide an engine for transcribing data into
[Lilypond](http://www.lilypond.org)-notated music. Certain design and pipeline
decisions are heavily informed by Ruby on Rails, but with the added assurance
of proper trait and type resolution that comes with Rust.

## Current design decisions

Currently, the `sequenza` module makes a few decisions that could be revised.

1. `Grouping` structs take a `Durational` template type that moves the whole
way up the hierarchy of `Grouping`s. This means that specific types cannot
currently be mixed. It would be possible to `Box` and mix `Durational` types,
which would use dynamic dispatch. It may also be possible to look into using
mixed types, some of which are stack-level `Durational` types that would be
homogeneous, and others that are `Box`ed and could be mixed.

2. Annotations are tied to the `Grouping`. Ideally, there would be nothing
hard-coded in the Rust source, and everything would be modifiable via
specification files or templates of some sort. It might be possible to take
a nod from Rails and use a combination of `yml` files for short segments (like
with the i18n gem) and templates for longer chunks.

## Suggestions

I highly recommend using `lyp` with this project. I anticipate using the server
capacity of `lyp` to render Lilypond scores in the background.

I have also worked to keep the data-processing sides of this application
distinct from the Lilypond sides. In other words, it should be possible to
translate the bulk of the specific generation methods to a format like
MusicXML, although I haven't tried to do so.

Also, you may notice there's basically no code here.


