use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
};

pub struct GeneratedCode {
    code: HashMap<PathBuf, String>,
}

impl GeneratedCode {
    pub fn new() -> Self {
        GeneratedCode {
            code: HashMap::new(),
        }
    }

    pub fn add_generated_code<G>(
        &mut self,
        path: &Path,
        code_generator: G,
    ) -> Result<(), std::io::Error>
    where
        G: Fn(&mut dyn Write) -> Result<(), std::io::Error>,
    {
        let mut code = Vec::new();
        code_generator(&mut code)?;
        let path_buf = path.to_path_buf();
        if self.code.contains_key(&path_buf) {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "duplicate path",
            ))
        } else {
            self.code.insert(
                path_buf,
                String::from_utf8(code)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?,
            );
            Ok(())
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Path, &str)> {
        self.code.iter().map(|(p, c)| (p.as_path(), c.as_str()))
    }
}
