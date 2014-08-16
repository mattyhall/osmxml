#![feature(globs)]
#![feature(struct_variant)]
extern crate sax;

use std::collections::HashMap;
use std::io::IoError;

pub type Tags = HashMap<String, String>;

#[deriving(Show)]
enum OsmParseError {
    IoErr(IoError),
    SaxErr(sax::error::ErrorData),
    ParseErr(&'static str),
}

pub type ParseResult = Result<(), OsmParseError>;

#[deriving(Show)]
pub enum OsmElement {
    Node{id: int, lat: f64, lng: f64, visible: bool, tags: Tags},
    Way{id: int, nodes: Vec<int>, tags: Tags},
}

pub struct Osm {
    pub parser: Receiver<sax::ParseResult>,
    pub elements: HashMap<int, OsmElement>,
}

impl Osm {
    pub fn new(path: &Path) -> Osm {
        let parser = sax::parse_file(path).unwrap();
        Osm {parser: parser, elements: HashMap::new()}
    }
    
    pub fn parse(&mut self) -> ParseResult {
        match self.parser.iter().next().unwrap() {
            Ok(sax::StartDocument) => (),
            Ok(_) => return Err(ParseErr("Document did not start with StartDocument event")),
            Err(e) => return Err(SaxErr(e))
        }

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => try!(self.parse_start_element(name, attrs)),
                Ok(_) => (),
                Err(e) => return Err(SaxErr(e)),
            }
        }
        Ok(())
    }

    fn parse_start_element(&mut self, name: String, attrs: sax::Attributes) -> ParseResult {
        match name.as_slice() {
            "relation" => try!(self.parse_relation()),
            "node" => try!(self.parse_node(attrs)),
            "way" => try!(self.parse_way(attrs)),
            _  => println!("Skipping {} tag", name)
        }
        Ok(())
    }

    fn parse_node(&mut self, attrs: sax::Attributes) -> ParseResult { 
        let id  = attrs.find("id").and_then(|v| from_str(v));
        let lat = attrs.find("lat").and_then(|v| from_str(v));
        let lng = attrs.find("lon").and_then(|v| from_str(v));
        let visible = attrs.find("visible").and_then(|v| from_str(v));
        let (id, lat, lng, visible) = match (id, lat, lng, visible) {
            (Some(id), Some(lat), Some(lng), Some(visible)) => (id, lat, lng, visible),
            _ => return Err(ParseErr("Could not find all required attributes on node"))
        };
        let mut tags = HashMap::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "tag" => try!(self.parse_tag(attrs, &mut tags)),
                        _ => return Err(ParseErr("Expecting all children of nodes to be tags")),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "node" {
                        break;
                    }
                    return Err(ParseErr("Expecting node to end"));
                }
                _ => {},
            }
        }

        self.elements.insert(id, Node{id: id, lat: lat, lng: lng, visible: visible, tags: tags});

        Ok(())
    }

    fn parse_way(&mut self, attrs: sax::Attributes) -> ParseResult {
        let id = from_str(attrs.get("id")).unwrap();
        let mut nodes = Vec::new();
        let mut tags = HashMap::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "nd" => nodes.push(try!(self.parse_nd(attrs))),
                        "tag" => try!(self.parse_tag(attrs, &mut tags)),
                        _ => return Err(ParseErr("Expecting children of way to be a nd or a tag")),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "way" {
                        break;
                    }
                    return Err(ParseErr("Expecting way to end"));
                }
                _ => (),
            }
        }
        self.elements.insert(id, Way {id: id, nodes: nodes, tags: tags});
        Ok(())
    }

    fn parse_nd(&mut self, attrs: sax::Attributes) -> Result<int, OsmParseError> {
        let i = from_str(attrs.get("ref")).unwrap();

        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "nd" {
                        break;
                    }
                    return Err(ParseErr("Expecting nd to end"));
                }
                _ => return Err(ParseErr("Expecting nd to end"))
            }
        }
        Ok(i)
    }

    fn parse_tag(&mut self, attrs: sax::Attributes, tags: &mut Tags) -> ParseResult {
        let (k, v) = match (attrs.find_clone("k"), attrs.find_clone("v")) {
            (Some(k), Some(v)) => (k, v),
            _ => return Err(ParseErr("Tag must have a k and a v attribute"))
        };
        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "tag" {
                        tags.insert(k, v);
                        return Ok(());
                    }
                    return Err(ParseErr("Expecting tag to end"));
                }
                _ => return Err(ParseErr("Expecting tag to end")),
            }
        }
        Ok(())
    }

    fn parse_relation(&mut self) -> ParseResult {

        Ok(())
    }
}
