use std::{collections::HashMap, io::Write};

mod templating;

pub use templating::Template;
pub use templating::TemplateWriter;

pub struct GeneratedCodeWriter<'writer> {
    targets: HashMap<&'static str, &'writer mut dyn Write>,
    default_writer_fun:
        Box<dyn (Fn(&'static str) -> std::io::Result<Box<dyn Write + 'writer>>) + 'writer>,
}

impl<'writer> GeneratedCodeWriter<'writer> {
    pub fn new() -> Self {
        GeneratedCodeWriter::with_default(|_| Ok(std::io::sink()))
    }

    pub fn with_default<F, W>(writer_fun: F) -> Self
    where
        W: Write + 'writer,
        F: (Fn(&'static str) -> std::io::Result<W>) + 'writer,
    {
        GeneratedCodeWriter {
            targets: HashMap::new(),
            default_writer_fun: Box::new(move |name| {
                let writer = writer_fun(name)?;
                Ok(Box::new(writer))
            }),
        }
    }

    pub fn add_target<W>(&mut self, key: &'static str, writer: &'writer mut W)
    where
        W: Write,
    {
        self.targets.insert(key, writer);
    }

    pub fn generate_code<G>(&mut self, key: &'static str, code_generator: G) -> std::io::Result<()>
    where
        G: Fn(&mut dyn Write) -> Result<(), std::io::Error>,
    {
        if let Some(writer) = self.targets.get_mut(&key) {
            code_generator(writer)
        } else {
            let mut sink = (self.default_writer_fun)(key)?;
            code_generator(&mut sink)
        }
    }
}
