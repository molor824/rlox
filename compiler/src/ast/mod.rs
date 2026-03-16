use crate::source::Source;

fn parse_digit(source: &mut Source, radix: u32) -> Option<u32> {
    let index = source.offset;
    match source.next().and_then(|ch| ch.to_digit(radix)) {
        Some(a) => Some(a),
        None => {
            source.offset = index;
            None
        }
    }
}