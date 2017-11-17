
use std;
use std::fmt::{self, Debug, Display, Formatter};

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Span {
	start: usize,
	end: usize
}


impl Debug for Span {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "{}..{}", self.start, self.end)
	}
}

impl Display for Span {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		if self.is_dummy() {
			write!(f, "<*>")
		} else if self.end - self.start <= 1 {
			write!(f, "{}", self.start)
		} else {
			Debug::fmt(self, f)
		}
	}
}

impl From<usize> for Span {
	fn from(pos: usize) -> Self {
		Span::new(pos, pos+1)
	}
}

impl From<(usize, usize)> for Span {
	fn from((start, end): (usize, usize)) -> Self {
		Span::new(start, end)
	}
}

impl Span {
	pub fn new(start: usize, end: usize) -> Self {
		debug_assert!(start > 0);
		debug_assert!(start <= end);
		Span{start, end}
	}

	pub fn dummy() -> Self {
		Span{start: 0, end: 0}
	}

	pub fn is_dummy(self) -> bool {
		self == Span::dummy()
	}

	pub fn start(self) -> usize {
		self.start
	}

	pub fn end(self) -> usize {
		self.end
	}

	pub fn contains(self, pos: usize) -> bool {
		self.start <= pos && pos < self.end
	}

	pub fn merge(self, other: Span) -> Self {
		if other.is_dummy() {
			return self;
		} else if self.is_dummy() {
			return other;
		}

		let start = std::cmp::min(self.start, other.start);
		let end = std::cmp::max(self.end, other.end);
		Span{start, end}
	}

	pub fn extend(self, pos: usize) -> Self {
		if pos == 0 {
			return self;
		} else if self.is_dummy() {
			return pos.into();
		}

		let mut s = self;
		if pos < s.start {
			s.start = pos;
		} else if pos >= s.end {
			s.end = pos+1;
		}
		s
	}

	pub fn shift(mut self, offset: usize) -> Self {
		if self.is_dummy() {
			return self;
		}
		self.start += offset;
		self.end += offset;
		self
	}

	pub fn with<T>(self, data: T) -> Spanned<T> {
		Spanned::new(data, self)
	}
}

#[derive(Copy, Clone, Debug)]
pub struct Spanned<T> {
	pub data: T,
	pub span: Span
}

impl<T: Display> Display for Spanned<T> {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		write!(f, "at {}: {}", self.span, self.data)
	}
}

macro_rules! impl_spanned_merge {
	($meth:ident; $($tname:ident, $vname:ident);+) => {
	impl<A> Spanned<A> {
		pub fn $meth <$($tname,)+ Fun, Out>(self, $($vname: Spanned<$tname>,)+ f: Fun) -> Spanned<Out>
			where Fun: FnOnce(A $(, $tname)+)-> Out {

			let data = f(self.data $(, $vname.data)+);
			let span = self.span $(.merge($vname.span))+ ;
			Spanned::new(data, span)
		}
	}};
}

impl<T> Spanned<T> {
	pub fn new(data: T, span: Span) -> Self {
		Spanned { data, span }
	}

	pub fn map<F, U>(self, f: F) -> Spanned<U>
		where F: FnOnce(T) -> U {
		Spanned::new(f(self.data), self.span)
	}

	pub fn merge<U, Out, Fun>(self, other: Spanned<U>, f: Fun) -> Spanned<Out>
		where Fun: FnOnce(T, U) -> Out {
		self.merge2(other, f)
	}
}

impl_spanned_merge!(merge2; B, b);
impl_spanned_merge!(merge3; B, b; C, c);
impl_spanned_merge!(merge4; B, b; C, c; D, d);
impl_spanned_merge!(merge5; B, b; C, c; D, d; E, e);
impl_spanned_merge!(merge6; B, b; C, c; D, d; E, e; F, f);
impl_spanned_merge!(merge7; B, b; C, c; D, d; E, e; F, f; G, g);
impl_spanned_merge!(merge8; B, b; C, c; D, d; E, e; F, f; G, g; H, h);

pub struct PrettyChar(pub char);

impl Display for PrettyChar {
	fn fmt(&self, f: &mut Formatter) -> fmt::Result {
		let c = self.0;
		match c {
			'\r' | '\t' | '\n' | ' ' => write!(f, "{:?}", c),
			'\\' | '"' => write!(f, "'{}'", c),
			'\'' => write!(f, "\"'\""),
			c if c.is_control() => write!(f, "{:#02X}", c as u32),
			_ => {
				let pad = match c as u32 {
					0...0x7F =>	0,
					0x80...0xFF => 2,
					0x100...0xFFFF => 4,
					_ => 6
				};
				if pad == 0 {
					write!(f, "{:?}", c)
				} else {
					write!(f, "{:?} ({:#0pad$X})", c, c as u32, pad = pad)	
				}
			} 
		}
	}
}