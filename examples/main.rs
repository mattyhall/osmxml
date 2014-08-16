extern crate osmxml;

use osmxml::Osm;

fn main() {
    let path = &Path::new("spa.osm");
    let mut osm = Osm::new(path);
    osm.parse();
    for (k, v) in osm.elements.iter() {
        println!("{}: {}", k, v);
    }
}
