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
        let id = from_str(attrs.get("id")).unwrap();
        let lat = from_str(attrs.get("lat")).unwrap();
        let lng = from_str(attrs.get("lon")).unwrap();
        let visible = from_str(attrs.get("visible")).unwrap();
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
        let (k, v) = (attrs.get_clone("k"), attrs.get_clone("v"));
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
