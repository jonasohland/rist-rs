#[macro_export]
macro_rules! cfg_alloc {
    ($($m:item)*) => {
        $(
            #[cfg(feature = "alloc")]
            $m
        )*
    };
}

#[macro_export]
macro_rules! cfg_no_alloc {
    ($($m:item)*) => {
        $(
            #[cfg(not(feature = "alloc"))]
            $m
        )*
    };
}

#[macro_export]
macro_rules! cfg_std {
    ($($m:item)*) => {
        $(
            #[cfg(feature = "std")]
            $m
        )*
    };
}

#[macro_export]
macro_rules! cfg_no_std {
    ($($m:item)*) => {
        $(
            #[cfg(not(feature = "std"))]
            $m
        )*
    };
}

#[macro_export]
macro_rules! cfg_serde {
    ($($m:item)*) => {
        $(
            #[cfg(feature = "serde")]
            $m
        )*
    };
}

#[macro_export]
macro_rules! cfg_no_serde {
    ($($m:item)*) => {
        $(
            #[cfg(not(feature = "serde"))]
            $m
        )*
    };
}

pub use cfg_alloc;
pub use cfg_no_alloc;
pub use cfg_no_serde;
pub use cfg_no_std;
pub use cfg_serde;
pub use cfg_std;
