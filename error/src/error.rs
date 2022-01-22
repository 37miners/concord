// Copyright 2021 The BMW Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use failure::{Backtrace, Context, Fail};
use std::array::TryFromSliceError;
use std::convert::Infallible;
use std::fmt;
use std::fmt::Display;
use std::num::ParseIntError;
use std::str::Utf8Error;
use std::string::FromUtf8Error;
use std::time::SystemTimeError;

/// Base Error struct which is used throught this crate and other crates
#[derive(Debug, Fail)]
pub struct Error {
	inner: Context<ErrorKind>,
}

/// Kinds of errors that can occur
#[derive(Clone, Eq, PartialEq, Debug, Fail)]
pub enum ErrorKind {
	/// DataError Error
	#[fail(display = "DataError Error: {}", _0)]
	DataError(String),
	/// LibRustlet Error
	#[fail(display = "LibRustletError Error: {}", _0)]
	LibRustletError(String),
	/// IO Error
	#[fail(display = "IO Error: {}", _0)]
	IOError(String),
	/// Not found Error
	#[fail(display = "Not found: {}", _0)]
	NotFoundErr(String),
	/// File Error
	#[fail(display = "File err: {}", _0)]
	FileErr(String),
	/// LMDB Error
	#[fail(display = "LMDB err: {}", _0)]
	LMDBError(String),
	/// UTF8 Error
	#[fail(display = "UTF8 err: {}", _0)]
	Utf8Error(String),
	/// CorruptedData Error
	#[fail(display = "Corrupted Data Error: {}", _0)]
	CorruptedData(String),
	/// Too large read error
	#[fail(display = "Too large read Error: {}", _0)]
	TooLargeReadErr(String),
	/// Count Error
	#[fail(display = "Count Error: {}", _0)]
	CountError(String),
	/// Unexpected Data
	#[fail(display = "Unexpected Data Error: {}", _0)]
	UnexpectedData(String),
	/// Unexpected Eof
	#[fail(display = "Unexpected Eof Error: {}", _0)]
	UnexpectedEof(String),
	/// Sort Error
	#[fail(display = "Sort Error: {}", _0)]
	SortError(String),
	/// Duplicate Error
	#[fail(display = "Duplicate Error: {}", _0)]
	DuplicateError(String),
	/// HexError
	#[fail(display = "Hex Error: {}", _0)]
	HexError(String),
	/// DecodeError
	#[fail(display = "Decode Error: {}", _0)]
	DecodeError(String),
	/// From Utf8 Error
	#[fail(display = "FromUtf8Error: {}", _0)]
	FromUtf8Error(String),
	/// ParseIntError
	#[fail(display = "ParseIntError: {}", _0)]
	ParseIntError(String),
	/// TryFromSliceError
	#[fail(display = "TryFromSliceError: {}", _0)]
	TryFromSliceError(String),
	/// SystemTimeError
	#[fail(display = "SystemTimeError: {}", _0)]
	SystemTimeError(String),
	/// Not Authorized
	#[fail(display = "Not Auth Error: {}", _0)]
	NotAuthorized(String),
	/// Infallible
	#[fail(display = "Infallible Error: {}", _0)]
	InfallibleError(String),
	/// Base58DecodeError
	#[fail(display = "Base58DecodeError: {}", _0)]
	Base58DecodeError(String),
	/// Lock error
	#[fail(display = "LockError: {}", _0)]
	LockError(String),
	/// ListenerNotFound
	#[fail(display = "ListenerNotFound: {}", _0)]
	ListenerNotFound(String),
	/// TorError
	#[fail(display = "TorError: {}", _0)]
	TorError(String),
	/// SerdeJsonError
	#[fail(display = "SerdeJsonError: {}", _0)]
	SerdeJsonError(String),
}

impl Display for Error {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let cause = match self.cause() {
			Some(c) => format!("{}", c),
			None => String::from("Unknown"),
		};
		let backtrace = match self.backtrace() {
			Some(b) => format!("{}", b),
			None => String::from("Unknown"),
		};
		let output = format!(
			"{} \n Cause: {} \n Backtrace: {}",
			self.inner, cause, backtrace
		);
		Display::fmt(&output, f)
	}
}

impl Error {
	/// get kind
	pub fn kind(&self) -> ErrorKind {
		self.inner.get_context().clone()
	}
	/// get cause
	pub fn cause(&self) -> Option<&dyn Fail> {
		self.inner.cause()
	}
	/// get backtrace
	pub fn backtrace(&self) -> Option<&Backtrace> {
		self.inner.backtrace()
	}
}

impl From<ErrorKind> for Error {
	fn from(kind: ErrorKind) -> Error {
		Error {
			inner: Context::new(kind),
		}
	}
}

impl From<librustlet::Error> for Error {
	fn from(e: librustlet::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::LibRustletError(format!("{}", e))),
		}
	}
}

impl From<std::io::Error> for Error {
	fn from(e: std::io::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::IOError(format!("{}", e))),
		}
	}
}

impl From<lmdb_zero::Error> for Error {
	fn from(e: lmdb_zero::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::LMDBError(e.to_string())),
		}
	}
}

impl From<Utf8Error> for Error {
	fn from(e: Utf8Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::Utf8Error(e.to_string())),
		}
	}
}

impl From<base64::DecodeError> for Error {
	fn from(e: base64::DecodeError) -> Error {
		Error {
			inner: Context::new(ErrorKind::DecodeError(e.to_string())),
		}
	}
}

impl From<FromUtf8Error> for Error {
	fn from(e: FromUtf8Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::FromUtf8Error(e.to_string())),
		}
	}
}

impl From<librustlet::ErrorKind> for Error {
	fn from(e: librustlet::ErrorKind) -> Error {
		Error {
			inner: Context::new(ErrorKind::LibRustletError(format!("{}", e))),
		}
	}
}

impl From<ParseIntError> for Error {
	fn from(e: ParseIntError) -> Error {
		Error {
			inner: Context::new(ErrorKind::ParseIntError(format!("{}", e))),
		}
	}
}

impl From<TryFromSliceError> for Error {
	fn from(e: TryFromSliceError) -> Error {
		Error {
			inner: Context::new(ErrorKind::TryFromSliceError(format!("{}", e))),
		}
	}
}

impl From<SystemTimeError> for Error {
	fn from(e: SystemTimeError) -> Error {
		Error {
			inner: Context::new(ErrorKind::SystemTimeError(format!("{}", e))),
		}
	}
}

impl From<Infallible> for Error {
	fn from(e: Infallible) -> Error {
		Error {
			inner: Context::new(ErrorKind::InfallibleError(format!("{}", e))),
		}
	}
}

impl From<bs58::decode::Error> for Error {
	fn from(e: bs58::decode::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::Base58DecodeError(format!("{}", e))),
		}
	}
}

impl From<serde_json::Error> for Error {
	fn from(e: serde_json::Error) -> Error {
		Error {
			inner: Context::new(ErrorKind::SerdeJsonError(format!("{}", e))),
		}
	}
}
