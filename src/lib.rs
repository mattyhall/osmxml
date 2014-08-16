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
    ParseErr(String),
}

pub type ParseResult = Result<(), OsmParseError>;

#[deriving(Show)]
pub enum OsmElement {
    Node{id: int, lat: f64, lng: f64, visible: bool, tags: Tags},
    Way{id: int, nodes: Vec<int>, tags: Tags},
    Relation{id: int, members: Vec<int>, tags: Tags},
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
            Ok(e) => return Err(ParseErr(format!("Document started with: {}", e))),
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
            "relation" => try!(self.parse_relation(attrs)),
            "node" => try!(self.parse_node(attrs)),
            "way" => try!(self.parse_way(attrs)),
            _  => ()
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
            _ => return Err(ParseErr(
                "Could not find all required attributes on node".to_string()))
        };
        let mut tags = HashMap::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "tag" => try!(self.parse_tag(attrs, &mut tags)),
                        _ => return Err(ParseErr(format!(
                              "Expecting all children of nodes to be tags. Got a {}",
                              name))),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "node" {
                        break;
                    }
                    return Err(ParseErr(format!(
                        "Expecting node to end, not a {}", name)));
                }
                _ => {},
            }
        }

        self.elements.insert(id, Node{id: id, lat: lat, lng: lng,
                                      visible: visible, tags: tags});

        Ok(())
    }

    fn parse_int_attr(&self, k: &str, attrs: sax::Attributes) -> Result<int, OsmParseError> {
        return match attrs.find(k).and_then(|v| from_str(v)) {
            Some(id) => Ok(id),
            None => Err(ParseErr("Could not find id/ref attribute".to_string())),
        };
    }

    fn parse_way(&mut self, attrs: sax::Attributes) -> ParseResult {
        let id = try!(self.parse_int_attr("id", attrs));
        let mut nodes = Vec::new();
        let mut tags = HashMap::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "nd" => nodes.push(try!(self.parse_nd(attrs))),
                        "tag" => try!(self.parse_tag(attrs, &mut tags)),
                        n => return Err(ParseErr(format!(
                            "Expecting children of way to be a nd or a tag. Instead got a {}",
                            n))),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "way" {
                        break;
                    }
                    return Err(ParseErr(format!(
                        "Expecting way to end. Instead got {}", name)));
                }
                _ => (),
            }
        }
        self.elements.insert(id, Way {id: id, nodes: nodes, tags: tags});
        Ok(())
    }

    fn parse_nd(&mut self, attrs: sax::Attributes) -> Result<int, OsmParseError> {
        let i = try!(self.parse_int_attr("ref", attrs));

        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "nd" {
                        break;
                    }
                    return Err(ParseErr(format!(
                        "Expecting nd to end. Instead got a {}", name)));
                }
                _ => return Err(ParseErr("Expecting nd to end".to_string()))
            }
        }
        Ok(i)
    }

    fn parse_tag(&mut self, attrs: sax::Attributes, tags: &mut Tags) -> ParseResult {
        let (k, v) = match (attrs.find_clone("k"), attrs.find_clone("v")) {
            (Some(k), Some(v)) => (k, v),
            _ => return Err(ParseErr("Tag must have a k and a v attribute".to_string()))
        };

        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "tag" {
                        tags.insert(k, v);
                        return Ok(());
                    }
                    return Err(ParseErr(format!(
                        "Expecting tag to end. Instead got a {}", name)));
                }
                _ => return Err(ParseErr("Expecting tag to end".to_string())),
            }
        }
        Ok(())
    }

    fn parse_member(&self, attrs: sax::Attributes) -> Result<int, OsmParseError> {
        let i = try!(self.parse_int_attr("ref", attrs));
        
        for event in self.parser.iter() {
            match event {
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "member" {
                        break
                    }

                    return Err(ParseErr(format!(
                        "Expecting tag to end. Instead got a {}", name)));
                }
                _ => return Err(ParseErr("Expecting tag to end".to_string())),
            }
        }

        Ok(i)
    }

    fn parse_relation(&mut self, attrs: sax::Attributes) -> ParseResult {
        let id = try!(self.parse_int_attr("id", attrs));
        let mut tags = HashMap::new();
        let mut members = Vec::new();

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        "member" => members.push(try!(self.parse_member(attrs))),
                        "tag" => try!(self.parse_tag(attrs, &mut tags)),
                        _ => return Err(ParseErr(format!(
                            "Relations can only have members and tags. Got {}",
                            name)))
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == "relation" {
                        break;
                    }
                    return Err(ParseErr(format!(
                        "Expecting tag to end. Instead got a {}", name)));
                }
                _ => ()
            }
        }

        self.elements.insert(id, Relation{id: id, members: members, tags: tags});
        Ok(())
    }
}
