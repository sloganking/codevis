use std::path::Path;

pub(crate) struct Cache<'syntax, 'theme> {
    syntax: &'syntax syntect::parsing::SyntaxSet,
    theme: &'theme syntect::highlighting::Theme,
    prev_syntax: usize,
}

impl<'a, 'b> Clone for Cache<'a, 'b> {
    fn clone(&self) -> Self {
        Cache {
            syntax: self.syntax,
            theme: self.theme,
            prev_syntax: self.prev_syntax,
        }
    }
}

impl<'syntax, 'theme> Cache<'syntax, 'theme> {
    pub fn new_with_plain_highlighter(
        syntax: &'syntax syntect::parsing::SyntaxSet,
        theme: &'theme syntect::highlighting::Theme,
    ) -> Self {
        let plain = syntax.find_syntax_plain_text();
        Cache {
            syntax,
            theme,
            prev_syntax: plain as *const _ as usize,
        }
    }

    pub fn new_plain_highlighter(&self) -> syntect::easy::HighlightLines<'theme> {
        syntect::easy::HighlightLines::new(self.syntax.find_syntax_plain_text(), self.theme)
    }
}

impl<'syntax, 'theme> Cache<'syntax, 'theme> {
    pub fn highlighter_for_file_name(
        &mut self,
        path: &Path,
    ) -> std::io::Result<Option<syntect::easy::HighlightLines<'theme>>> {
        let syntax = self
            .syntax
            .find_syntax_for_file(path)
            .unwrap()
            .unwrap_or_else(|| self.syntax.find_syntax_plain_text());
        if syntax as *const _ as usize != self.prev_syntax {
            self.prev_syntax = syntax as *const _ as usize;
            Ok(Some(syntect::easy::HighlightLines::new(syntax, self.theme)))
        } else {
            Ok(None)
        }
    }
}
