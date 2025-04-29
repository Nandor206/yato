// =============== Imports ================
use std::fmt;
use console::{ style, Style };
use fuzzy_matcher::{ skim::SkimMatcherV2, FuzzyMatcher };
use dialoguer::theme::Theme;

// Base: Dialoguer Simple Theme
// Modified to my liking, removed the unused stuff
// Changes are noted mostly in the comments

pub struct CustomTheme {}

impl Theme for CustomTheme {
    // Prompts are going to be Bold and Cyan
    fn format_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        let prompt_style = Style::new().cyan().bold();
        write!(f, "{}", prompt_style.apply_to(prompt))
    }

    // Left on default settings
    fn format_error(&self, f: &mut dyn fmt::Write, err: &str) -> fmt::Result {
        log::debug!("Formatting error: {}", err);
        write!(f, "error: {}", err)
    }

    // Left on default settings
    fn format_select_prompt(&self, f: &mut dyn fmt::Write, prompt: &str) -> fmt::Result {
        self.format_prompt(f, prompt)
    }

    // Left on default settings
    fn format_select_prompt_selection(
        &self,

        f: &mut dyn fmt::Write,

        prompt: &str,

        sel: &str
    ) -> fmt::Result {
        self.format_input_prompt_selection(f, prompt, sel)
    }

    // Extra arrow added
    fn format_select_prompt_item(
        &self,

        f: &mut dyn fmt::Write,

        text: &str,

        active: bool
    ) -> fmt::Result {
        write!(f, "{} {}", if active { format!("{}", style(">>").bold()) } else { " ".to_string() }, text)
    }

    // Extra arrow added
    fn format_fuzzy_select_prompt_item(
        &self,

        f: &mut dyn fmt::Write,

        text: &str,

        active: bool,

        highlight_matches: bool,

        matcher: &SkimMatcherV2,

        search_term: &str
    ) -> fmt::Result {
        write!(f, "{} ", if active { format!("{}", style(">>").bold()) } else { " ".to_string() })?;

        if highlight_matches {
            if let Some((_score, indices)) = matcher.fuzzy_indices(text, search_term) {
                for (idx, c) in text.chars().enumerate() {
                    if indices.contains(&idx) {
                        write!(f, "{}", style(c).for_stderr().bold())?;
                    } else {
                        write!(f, "{}", c)?;
                    }
                }

                return Ok(());
            }
        }

        write!(f, "{}", text)
    }

    // Removed | from the search term, prompt is now bold and cyan
    fn format_fuzzy_select_prompt(
        &self,

        f: &mut dyn fmt::Write,

        prompt: &str,

        search_term: &str,

        bytes_pos: usize
    ) -> fmt::Result {
        let prompt_style = Style::new().cyan().bold();
        if !prompt.is_empty() {
            write!(f, "{} ", prompt_style.apply_to(prompt))?;
        }

        let (st_head, st_tail) = search_term.split_at(bytes_pos);

        write!(f, "{st_head} {st_tail}")
    }

    // Prompts are Bold and Cyan
    fn format_input_prompt(

        &self,

        f: &mut dyn fmt::Write,

        prompt: &str,

        default: Option<&str>,

    ) -> fmt::Result {
        let prompt_style = Style::new().cyan().bold();
        match default {

            Some(default) if prompt.is_empty() => write!(f, "[{}]: ", default),

            Some(default) => write!(f, "{} [{}]: ", prompt_style.apply_to(prompt), default),

            None => write!(f, "{}: ", prompt_style.apply_to(prompt)),

        }

    }

    // Active items are now bold and cyan
    fn format_multi_select_prompt_item(
        &self,
        f: &mut dyn fmt::Write,
        text: &str,
        checked: bool,
        active: bool,
    ) -> fmt::Result {
        let prefix = if active {
            Style::new().bold().apply_to(">>").to_string()
        } else {
            " ".to_string()
        };
    
        let styled_text = if checked {
            Style::new().bold().cyan().apply_to(text).to_string()
        } else {
            text.to_string()
        };
    
        write!(f, "{} {}", prefix, styled_text)
    }    
}
