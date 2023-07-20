use std::{
    collections::HashMap,
    io::{Error, Write},
};

use regex::{Captures, Regex};

pub struct GeneratedCodeWriter<'writer> {
    targets: HashMap<&'static str, &'writer mut dyn Write>,
    default_writer_fun: Box<dyn (Fn(&'static str) -> Box<dyn Write + 'writer>) + 'writer>,
}

impl<'writer> GeneratedCodeWriter<'writer> {
    pub fn new() -> Self {
        GeneratedCodeWriter::with_default(|_| std::io::sink())
    }

    pub fn with_default<F, W>(writer_fun: F) -> Self
    where
        W: Write + 'writer,
        F: (Fn(&'static str) -> W) + 'writer,
    {
        GeneratedCodeWriter {
            targets: HashMap::new(),
            default_writer_fun: Box::new(move |name| {
                let writer = writer_fun(name);
                Box::new(writer)
            }),
        }
    }

    pub fn add_target<W>(&mut self, key: &'static str, writer: &'writer mut W)
    where
        W: Write,
    {
        self.targets.insert(key, writer);
    }

    pub fn generate_code<G>(
        &mut self,
        key: &'static str,
        code_generator: G,
    ) -> Result<(), std::io::Error>
    where
        G: Fn(&mut dyn Write) -> Result<(), std::io::Error>,
    {
        if let Some(writer) = self.targets.get_mut(&key) {
            code_generator(writer)
        } else {
            let mut sink = (self.default_writer_fun)(key);
            code_generator(&mut sink)
        }
    }
}

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
        let template_regex = Regex::new("\\/\\*\\{(.*?)\\}\\*\\/").unwrap();
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
