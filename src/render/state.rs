use image::Rgb;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use syntect::highlighting::Style;

#[allow(dead_code)]
pub(crate) struct State<'syntax, 'theme, Container> {
    syntax: &'syntax syntect::parsing::SyntaxSet,
    theme: &'theme syntect::highlighting::Theme,
    highlighter: syntect::easy::HighlightLines<'theme>,
    prev_syntax: *const syntect::parsing::SyntaxReference,
    img: image::ImageBuffer<Rgb<u8>, Container>,
}

#[allow(dead_code)]
impl<'syntax, 'theme, Container> State<'syntax, 'theme, Container>
where
    Container: Deref<Target = [u8]>,
    Container: DerefMut,
{
    pub fn new_with_plain_highlighter(
        imgx: u32,
        imgy: u32,
        syntax: &'syntax syntect::parsing::SyntaxSet,
        theme: &'theme syntect::highlighting::Theme,
        container: Container,
    ) -> Self {
        let plain = syntax.find_syntax_plain_text();
        State {
            syntax,
            theme,
            prev_syntax: plain as *const _,
            highlighter: syntect::easy::HighlightLines::new(plain, theme),
            img: image::ImageBuffer::<Rgb<u8>, _>::from_raw(imgx, imgy, container)
                .expect("suitable image size and container"),
        }
    }
}

#[allow(dead_code)]
impl<Container> State<'_, '_, Container> {
    pub fn change_highlighter_by_file_name(&mut self, path: &Path) -> std::io::Result<()> {
        let syntax = self
            .syntax
            .find_syntax_for_file(&path)?
            .unwrap_or_else(|| self.syntax.find_syntax_plain_text());
        if syntax as *const _ != self.prev_syntax {
            self.highlighter = syntect::easy::HighlightLines::new(syntax, self.theme);
            self.prev_syntax = syntax as *const _;
        }
        Ok(())
    }

    pub fn highlight_line<'a>(
        &mut self,
        line: &'a str,
    ) -> Result<Vec<(Style, &'a str)>, syntect::Error> {
        self.highlighter.highlight_line(line, self.syntax)
    }
}
