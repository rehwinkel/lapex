use std::{
    collections::HashMap,
    io::{Error, Write},
};

use once_cell::unsync::Lazy;
use regex::{Captures, Regex};

pub struct Template<'src> {
    source: &'src str,
}

impl<'src> Template<'src> {
    pub fn new(source: &'src str) -> Self {
        Template { source }
    }

    pub fn writer<'writer>(&'src self) -> TemplateWriter<'writer, 'src> {
        TemplateWriter {
            template: self.source,
            substitutions: HashMap::new(),
        }
    }
}

pub struct TemplateWriter<'writer, 'template> {
    template: &'template str,
    substitutions:
        HashMap<&'static str, Box<dyn Fn(&mut dyn Write) -> Result<(), Error> + 'writer>>,
}

impl<'writer, 'template> TemplateWriter<'writer, 'template> {
    pub fn substitute<F>(&mut self, key: &'static str, writer: F)
    where
        F: Fn(&mut dyn Write) -> Result<(), std::io::Error> + 'writer,
    {
        self.substitutions.insert(key, Box::new(writer));
    }

    fn insert_substitution(&self, key: &str, writer: &mut dyn Write) -> Result<(), Error> {
        let subsitution_function = if let Some(fun) = self.substitutions.get(key) {
            fun
        } else {
            panic!(
                "Substitution of '{}' in template failed, as no substitution for it was supplied",
                key
            );
        };
        subsitution_function(writer)
    }
}

fn replace_all_streaming<'h, R>(
    regex: &Regex,
    haystack: &'h str,
    rep: R,
    writer: &mut dyn Write,
) -> std::io::Result<()>
where
    R: Fn(&Captures, &mut dyn Write) -> std::io::Result<()>,
{
    let mut it = regex.captures_iter(haystack).peekable();
    if it.peek().is_none() {
        write!(writer, "{}", haystack)?;
        return Ok(());
    }
    let mut last_match = 0;
    for cap in it {
        // unwrap on 0 is OK because captures only reports matches
        let m = cap.get(0).unwrap();
        write!(writer, "{}", &haystack[last_match..m.start()])?;
        rep(&cap, writer)?;
        last_match = m.end();
    }
    write!(writer, "{}", &haystack[last_match..])
}

impl<'writer, 'template> TemplateWriter<'writer, 'template> {
    pub fn write(&self, f: &mut dyn Write) -> std::io::Result<()> {
        let template_regex: Lazy<Regex> =
            Lazy::new(|| Regex::new("\\/\\*\\{(.*?)\\}\\*\\/").unwrap());
        replace_all_streaming(
            &template_regex,
            self.template,
            |captures, writer| {
                let key = captures.get(1).unwrap().as_str().trim();
                self.insert_substitution(key, writer)
            },
            f,
        )?;
        Ok(())
    }
}
