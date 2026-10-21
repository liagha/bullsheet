use masonry::app::RenderRoot;
use masonry::core::BrushIndex;
use masonry::parley::fontique::Blob;
use masonry::parley::style::{FontFamily, FontStack, GenericFamily};
use masonry::parley::{FontContext, Layout, LayoutContext};

use crate::writing::Writing;

const JET_BRAINS_MONO: &[u8] = include_bytes!("../assets/JetBrainsMono-Regular.ttf");
const VAZIRMATN: &[u8] = include_bytes!("../assets/Vazirmatn-Regular.ttf");

pub struct Typefaces {
    latin: FontStack<'static>,
    persian: FontStack<'static>,
    fonts: Vec<&'static [u8]>,
}

impl Typefaces {
    pub fn new() -> Self {
        Self {
            latin: FontStack::List(std::borrow::Cow::Owned(vec![
                FontFamily::Named("JetBrains Mono".into()),
                FontFamily::Named("Vazirmatn".into()),
                FontFamily::Generic(GenericFamily::SansSerif),
            ])),
            persian: FontStack::List(std::borrow::Cow::Owned(vec![
                FontFamily::Named("Vazirmatn".into()),
                FontFamily::Named("JetBrains Mono".into()),
                FontFamily::Generic(GenericFamily::SansSerif),
            ])),
            fonts: vec![JET_BRAINS_MONO, VAZIRMATN],
        }
    }

    pub fn register(&self, root: &mut RenderRoot) {
        for font in &self.fonts {
            let _ = root.register_fonts(Blob::from(font.to_vec()));
        }
    }

    pub fn stack(&self, writing: Writing) -> &FontStack<'static> {
        match writing {
            Writing::Latin => &self.latin,
            Writing::Persian => &self.persian,
        }
    }

    pub fn layout(
        &self,
        writing: Writing,
        fonts: &mut FontContext,
        layouts: &mut LayoutContext<BrushIndex>,
        text: &str,
        size: f32,
    ) -> Layout<BrushIndex> {
        let mut builder = layouts.ranged_builder(fonts, text, 1.0, true);
        builder.push_default(masonry::parley::style::StyleProperty::FontStack(
            self.stack(writing).clone(),
        ));
        builder.push_default(masonry::parley::style::StyleProperty::FontSize(size));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    }
}
