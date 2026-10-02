use crate::{diag, known_values};

/// Every CSS property the engine knows by name. Unknown names are allowed (open-world),
/// but a name that looks like a typo of one of these is reported with a suggestion.
const PROPERTY_NAMES: &str = "
accent-color align-content align-items align-self all animation animation-delay animation-direction
animation-duration animation-fill-mode animation-iteration-count animation-name animation-play-state
animation-timing-function appearance aspect-ratio backdrop-filter backface-visibility background
background-attachment background-blend-mode background-clip background-color background-image
background-origin background-position background-position-x background-position-y background-repeat
background-size block-size border border-block border-block-color border-block-end border-block-start
border-block-style border-block-width border-bottom border-bottom-color border-bottom-left-radius
border-bottom-right-radius border-bottom-style border-bottom-width border-collapse border-color
border-end-end-radius border-end-start-radius border-image border-inline border-inline-color
border-inline-end border-inline-start border-inline-style border-inline-width border-left
border-left-color border-left-style border-left-width border-radius border-right border-right-color
border-right-style border-right-width border-spacing border-start-end-radius border-start-start-radius
border-style border-top border-top-color border-top-left-radius border-top-right-radius border-top-style
border-top-width border-width bottom box-decoration-break box-shadow box-sizing break-after break-before
break-inside caption-side caret-color clear clip clip-path color color-scheme column-count column-fill
column-gap column-rule column-rule-color column-rule-style column-rule-width column-span column-width
columns contain container container-name container-type content content-visibility counter-increment
counter-reset cursor direction display empty-cells filter flex flex-basis flex-direction flex-flow
flex-grow flex-shrink flex-wrap float font font-family font-feature-settings font-kerning
font-optical-sizing font-size font-size-adjust font-stretch font-style font-variant font-variant-numeric
font-variation-settings font-weight gap grid grid-area grid-auto-columns grid-auto-flow grid-auto-rows
grid-column grid-column-end grid-column-start grid-row grid-row-end grid-row-start grid-template
grid-template-areas grid-template-columns grid-template-rows hanging-punctuation height hyphens
image-rendering inline-size inset inset-block inset-inline isolation justify-content justify-items
justify-self left letter-spacing line-break line-height list-style list-style-image list-style-position
list-style-type margin margin-block margin-block-end margin-block-start margin-bottom margin-inline
margin-inline-end margin-inline-start margin-left margin-right margin-top mask mask-image mask-position
mask-repeat mask-size max-block-size max-height max-inline-size max-width min-block-size min-height
min-inline-size min-width mix-blend-mode object-fit object-position offset opacity order orphans outline
outline-color outline-offset outline-style outline-width overflow overflow-anchor overflow-wrap
overflow-x overflow-y overscroll-behavior overscroll-behavior-x overscroll-behavior-y padding
padding-block padding-block-end padding-block-start padding-bottom padding-inline padding-inline-end
padding-inline-start padding-left padding-right padding-top page-break-after page-break-before
page-break-inside perspective perspective-origin place-content place-items place-self pointer-events
position quotes resize right rotate row-gap scale scroll-behavior scroll-margin scroll-padding
scroll-snap-align scroll-snap-stop scroll-snap-type scrollbar-color scrollbar-gutter scrollbar-width
shape-outside tab-size table-layout text-align text-align-last text-decoration text-decoration-color
text-decoration-line text-decoration-style text-decoration-thickness text-indent text-justify
text-overflow text-shadow text-transform text-underline-offset text-underline-position text-wrap top
touch-action transform transform-box transform-origin transform-style transition transition-delay
transition-duration transition-property transition-timing-function translate unicode-bidi user-select
vertical-align view-transition-name visibility white-space widows width will-change word-break
word-spacing word-wrap writing-mode z-index zoom
";

const HINTS: &[(&str, &str)] = &[
    ("width", "<length> | <percentage> | auto | fit-content | min-content | max-content"),
    ("height", "<length> | <percentage> | auto | fit-content | min-content | max-content"),
    ("padding", "<length> | <percentage>  (1 to 4 values)"),
    ("margin", "<length> | <percentage> | auto  (1 to 4 values)"),
    ("gap", "<length> | <percentage>  (row column)"),
    ("color", "<color>  (quote hex colors: \"#fff\")"),
    ("background", "<color> | <image> | linear-gradient(…)  (quote hex colors)"),
    ("border", "<width> <style> <color>   e.g. 1px solid var(--border)"),
    ("border-radius", "<length> | <percentage>  (1 to 4 values)"),
    ("box-shadow", "<x> <y> <blur> <spread> <color>, …  (inset allowed)"),
    ("opacity", "0 to 1"),
    ("z-index", "<integer> | auto"),
    ("font-size", "<length> | <percentage> | small | medium | large …"),
    ("line-height", "<number> | <length> | normal"),
    ("letter-spacing", "<length> | normal"),
    ("transition", "<property> <duration> <easing> <delay>, …"),
    ("animation", "<name> <duration> <easing> <delay> <count> <direction> <fill>, …  (names normalize _ to -)"),
    ("transform", "translate(…) scale(…) rotate(…) …"),
    ("flex", "<grow> <shrink> <basis> | auto | none"),
    ("grid-template-columns", "repeat(auto-fill, minmax(150px, 1fr)) | <track-list>"),
    ("aspect-ratio", "<ratio>   e.g. 1 / 1, 16 / 9"),
    ("inset", "<length> | <percentage> | auto  (1 to 4 values)"),
    ("top", "<length> | <percentage> | auto"),
    ("left", "<length> | <percentage> | auto"),
    ("right", "<length> | <percentage> | auto"),
    ("bottom", "<length> | <percentage> | auto"),
];

pub fn all() -> impl Iterator<Item = &'static str> {
    PROPERTY_NAMES.split_whitespace()
}

pub fn is_known(name: &str) -> bool {
    all().any(|p| p == name)
}

/// A close, known property for a name that is not itself known.
pub fn suggest(name: &str) -> Option<&'static str> {
    if name.starts_with('-') || is_known(name) {
        return None;
    }
    let max = if name.len() >= 5 {
        2
    } else if name.len() >= 4 {
        1
    } else {
        return None;
    };
    diag::closest(name, all(), max)
}

pub fn doc(name: &str) -> String {
    if let Some(values) = known_values::values_for(name) {
        return format!("{name}: {}", values.join(" | "));
    }
    match HINTS.iter().find(|(n, _)| *n == name) {
        Some((_, hint)) => format!("{name}: {hint}"),
        None => name.to_string(),
    }
}