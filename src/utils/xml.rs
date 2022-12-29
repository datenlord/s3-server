//! helper trait for writing xml

use std::io;
use std::ops::Deref;
use xml::writer::{events::XmlEvent, EventWriter, Result};

/// helper trait for writing xml
pub trait XmlWriterExt {
    /// write xml stack
    fn stack(&mut self, name: &str, f: impl FnOnce(&mut Self) -> Result<()>) -> Result<()>;

    /// write xml optional stack
    fn opt_stack<T>(
        &mut self,
        name: &str,
        data: Option<T>,
        f: impl FnOnce(&mut Self, T) -> Result<()>,
    ) -> Result<()>;

    /// write xml element
    fn element(&mut self, name: &str, data: &str) -> Result<()>;

    /// write xml optional element
    fn opt_element(&mut self, name: &str, data: Option<impl Deref<Target = str>>) -> Result<()>;

    /// write xml by an iterator
    fn iter_element<T>(
        &mut self,
        iter: impl Iterator<Item = T>,
        f: impl FnMut(&mut Self, T) -> Result<()>,
    ) -> Result<()>;
}

impl<W: io::Write> XmlWriterExt for EventWriter<W> {
    fn stack(&mut self, name: &str, f: impl FnOnce(&mut Self) -> Result<()>) -> Result<()> {
        self.write(XmlEvent::start_element(name))?;
        f(self)?;
        self.write(XmlEvent::end_element())
    }

    fn opt_stack<T>(
        &mut self,
        name: &str,
        data: Option<T>,
        f: impl FnOnce(&mut Self, T) -> Result<()>,
    ) -> Result<()> {
        if let Some(data) = data {
            self.write(XmlEvent::start_element(name))?;
            f(self, data)?;
            self.write(XmlEvent::end_element())?;
        }
        Ok(())
    }

    fn element(&mut self, name: &str, data: &str) -> Result<()> {
        self.write(XmlEvent::start_element(name))?;
        self.write(XmlEvent::characters(data))?;
        self.write(XmlEvent::end_element())
    }

    fn opt_element(&mut self, name: &str, data: Option<impl Deref<Target = str>>) -> Result<()> {
        if let Some(data) = data {
            self.write(XmlEvent::start_element(name))?;
            self.write(XmlEvent::characters(&data))?;
            self.write(XmlEvent::end_element())?;
        }
        Ok(())
    }

    fn iter_element<T>(
        &mut self,
        iter: impl Iterator<Item = T>,
        mut f: impl FnMut(&mut Self, T) -> Result<()>,
    ) -> Result<()> {
        for data in iter {
            f(self, data)?;
        }
        Ok(())
    }
}
