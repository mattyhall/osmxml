#![feature(globs, macro_rules, struct_variant)]
extern crate sax;

use std::collections::HashMap;
use std::io::IoError;

pub type Tags = HashMap<String, String>;

#[deriving(Show)]
pub enum OsmParseError {
    IoErr(IoError),
    SaxErr(sax::error::ErrorData),
    ParseErr(String),
}

pub type ParseResult = Result<(), OsmParseError>;

#[deriving(Show)]
pub enum OsmElement {
    Node {
        pub id: int,
        pub lat: f64, pub lng: f64,
        pub visible: bool, pub tags: Tags
    },
    Way { pub id: int, pub nodes: Vec<int>, pub tags: Tags },
    Relation { pub id: int, pub members: Vec<int>, pub tags: Tags },
}

macro_rules! parse {
    ($iter:expr, $close_tag:expr $(, $tag:pat => $method:expr)*) => {
        for event in $iter {
            match event {
                Ok(sax::StartElement(name, attrs)) => {
                    match name.as_slice() {
                        $($tag => try!($method(attrs)),)*
                        _ => return Err(ParseErr(format!(
                              "Unexpected child in {} Got a {}",
                              $close_tag, name))),
                    }
                }
                Ok(sax::EndElement(name)) => {
                    if name.as_slice() == $close_tag {
                        break;
                    }
                    return Err(ParseErr(format!(
                            "Expecting {} to end, not a {}", 
                            $close_tag, name)));
                }
                _ => {},
            }
        }
    }
}

pub struct Osm {
    parser: Receiver<sax::ParseResult>,
    pub elements: HashMap<int, OsmElement>,
}

impl Osm {
    pub fn new(path: &Path) -> Result<Osm, OsmParseError> {
        let parser = sax::parse_file(path).unwrap();
        let mut s = Osm {parser: parser, elements: HashMap::new()};
        try!(s.parse());
        Ok(s)
    }
    
    fn parse(&mut self) -> ParseResult {
        match self.parser.recv() {
            Ok(sax::StartDocument) => (),
            Ok(e) => return Err(ParseErr(format!("Document started with: {}", e))),
            Err(e) => return Err(SaxErr(e))
        }

        for event in self.parser.iter() {
            match event {
                Ok(sax::StartElement(name, attrs)) => try!(self.parse_start_element(name, attrs)),
                // Should only skip comments/text as each parse function is responsible for
                // matching it's ending event
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

        parse!(self.parser.iter(), "node", 
               "tag" => |attrs| self.parse_tag(attrs, &mut tags));

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

        parse!(self.parser.iter(), "way",
               "nd" => |attrs| Ok(nodes.push(try!(self.parse_nd(attrs)))),
               "tag" => |attrs| self.parse_tag(attrs, &mut tags));

        self.elements.insert(id, Way {id: id, nodes: nodes, tags: tags});
        Ok(())
    }

    fn parse_nd(&mut self, attrs: sax::Attributes) -> Result<int, OsmParseError> {
        let i = try!(self.parse_int_attr("ref", attrs));
        parse!(self.parser.iter(), "nd");
        Ok(i)
    }

    fn parse_tag(&mut self, attrs: sax::Attributes, tags: &mut Tags) -> ParseResult {
        let (k, v) = match (attrs.find_clone("k"), attrs.find_clone("v")) {
            (Some(k), Some(v)) => (k, v),
            _ => return Err(ParseErr("Tag must have a k and a v attribute".to_string()))
        };

        parse!(self.parser.iter(), "tag");

        tags.insert(k, v);
        Ok(())
    }

    fn parse_member(&self, attrs: sax::Attributes) -> Result<int, OsmParseError> {
        let i = try!(self.parse_int_attr("ref", attrs));

        parse!(self.parser.iter(), "member");

        Ok(i)
    }

    fn parse_relation(&mut self, attrs: sax::Attributes) -> ParseResult {
        let id = try!(self.parse_int_attr("id", attrs));
        let mut tags = HashMap::new();
        let mut members = Vec::new();

        parse!(self.parser.iter(), "relation",
               "member" => |attrs| Ok(members.push(try!(self.parse_member(attrs)))),
                "tag" => |attrs| self.parse_tag(attrs, &mut tags));

        self.elements.insert(id, Relation{id: id, members: members, tags: tags});
        Ok(())
    }
}
