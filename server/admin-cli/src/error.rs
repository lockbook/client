pub type CliResult<T> = Result<T, Error>;

pub enum Error {
    Simple(ErrorKind),
    Custom(CustomError),
}

impl Error {
    pub fn print(&self) {
        match self {
            Self::Simple(base) => eprintln!("error: {}", base.msg()),
            Self::Custom(err) => {
                let base_msg = err.base.msg();
                let with_ctx = match &err.context {
                    Some(ctx) => format!("{}: {}", ctx, base_msg),
                    None => base_msg,
                };

                eprintln!("error: {}", with_ctx);
                if let Some(extra) = &err.extra {
                    eprintln!("{}", extra);
                }
            }
        }
    }

    pub fn exit(&self) -> ! {
        self.print();
        std::process::exit(match self {
            Self::Simple(base) => base.code(),
            Self::Custom(err) => err.base.code(),
        })
    }
}

macro_rules! underscore {
    ($t:ty) => { _ };
}

macro_rules! make_errkind_enum {
    ($( $codes:literal => $variants:ident $( ( $( $types:ty ),* ) )? ,)*) => {
        pub enum ErrorKind {
            $( $variants $( ( $( $types ),* ) )?, )*
        }

        impl ErrorKind {
            pub fn code(&self) -> i32 {
                match self {
                    $( Self::$variants $( ( $( underscore!($types) ),* ) )? => $codes ,)*
                }
            }

            pub fn print_table() -> CliResult<()> {
                $( println!("{:>6}  {}", $codes, stringify!($variants)); )*
                Ok(())
            }
        }
    };
}

type IoError = std::io::Error;

// Error Codes, respect: http://www.tldp.org/LDP/abs/html/exitcodes.html
make_errkind_enum!(
    // Miscellaneous (3-19)
    3 => Unexpected(String),
    4 => InvalidCredentials(String),
    5 => NetworkIssues,
    6 => PostgressError(String),
    7 => FilesDbError(String),
);

impl ErrorKind {
    pub fn msg(&self) -> String {
        match self {
            Self::Unexpected(msg) => msg.to_string(),
            ErrorKind::InvalidCredentials(err) => format!("Your credentials were invalid: {}", err),
            ErrorKind::NetworkIssues => "Could not reach Postgres and/or FilesDb!".to_string(),
            ErrorKind::PostgressError(err) => format!("You were returned a postgres error: {}", err),
            ErrorKind::FilesDbError(err) => format!("You were returned a files db error: {}", err),
        }
    }
}
