//! A simplified view of XML for the Bible importers.
//!
//! Bible formats only need three things from XML: where elements start and
//! end, their attributes, and the text between them. This module turns
//! quick-xml's detailed events into exactly that, with element and
//! attribute names in lowercase (Zefania writes `<BIBLEBOOK>`, others
//! `<book>`) and entities such as `&amp;` already resolved.

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::{Reader, XmlVersion};

use super::ImportError;

/// One simplified XML event.
#[derive(Debug)]
pub enum XmlEvent {
    /// An element starts. `empty` is true for self-closing elements such as
    /// `<verse sID="Gen.1.1"/>`, which have no matching [`XmlEvent::End`].
    Start {
        name: String,
        attributes: HashMap<String, String>,
        empty: bool,
    },
    /// A (non-empty) element ends.
    End { name: String },
    /// Text content.
    Text(String),
}

/// Calls `handle` for every event in `xml`, in document order.
pub fn for_each_event(
    xml: &str,
    mut handle: impl FnMut(XmlEvent) -> Result<(), ImportError>,
) -> Result<(), ImportError> {
    let mut reader = Reader::from_str(xml);
    loop {
        let event = reader
            .read_event()
            .map_err(|error| ImportError::Malformed {
                reason: error.to_string(),
            })?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let name = lowercase_name(element.local_name().as_ref());
                let attributes = element
                    .attributes()
                    .flatten()
                    .map(|attribute| {
                        let key = lowercase_name(attribute.key.local_name().as_ref());
                        let value = attribute
                            .normalized_value(XmlVersion::Implicit1_0)
                            .map(|value| value.into_owned())
                            .unwrap_or_default();
                        (key, value)
                    })
                    .collect();
                handle(XmlEvent::Start {
                    name,
                    attributes,
                    empty,
                })?;
            }
            Event::End(element) => handle(XmlEvent::End {
                name: lowercase_name(element.local_name().as_ref()),
            })?,
            Event::Text(text) => handle(XmlEvent::Text(text.xml10_content().into_owned()))?,
            Event::CData(data) => handle(XmlEvent::Text(data.xml10_content().into_owned()))?,
            Event::GeneralRef(reference) => {
                if let Some(text) = resolve_reference(&reference.clone().into_inner()) {
                    handle(XmlEvent::Text(text))?;
                }
            }
            Event::Eof => return Ok(()),
            // Declarations, comments, processing instructions, doctypes.
            _ => {}
        }
    }
}

/// Resolves `amp`, `#169`, `#xA9` and the other predefined entities.
/// Unknown named entities (from a DTD we do not read) are dropped.
fn resolve_reference(name: &str) -> Option<String> {
    if let Some(number) = name.strip_prefix('#') {
        let code = match number.strip_prefix(['x', 'X']) {
            Some(hex) => u32::from_str_radix(hex, 16).ok()?,
            None => number.parse().ok()?,
        };
        return char::from_u32(code).map(String::from);
    }
    quick_xml::escape::resolve_predefined_entity(name).map(str::to_owned)
}

fn lowercase_name(name: &str) -> String {
    name.to_lowercase()
}

/// The value of attribute `name`, if present.
pub fn attribute<'a>(attributes: &'a HashMap<String, String>, name: &str) -> Option<&'a str> {
    attributes.get(name).map(String::as_str)
}
