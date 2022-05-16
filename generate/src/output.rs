use std::fmt;

pub struct Output(String);

impl Output {
    pub fn new() -> Self {
        Output(String::new())
    }

    pub fn write_fmt(&mut self, arguments: fmt::Arguments) {
        fmt::Write::write_fmt(&mut self.0, arguments).unwrap();
    }
}

impl AsRef<[u8]> for Output {
    fn as_ref(&self) -> &[u8] {
        self.0.as_bytes()
    }
}
