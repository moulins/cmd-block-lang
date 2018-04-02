
pub mod text;
pub mod interning;

pub fn is_numeric(c: u8) -> bool {
	match c {
		b'0'...b'9' => true,
		_ => false
	}
}

pub fn is_whitespace(c: u8) -> bool {
	match c {
		b' ' | b'\t' | b'\n' | b'\r' => true,
		_ => false
	}
}

pub fn is_identifier_start(c: u8) -> bool {
	match c {
		b'a'...b'z' | b'A'...b'Z' | b'_' => true,
		_ => false
	}
}