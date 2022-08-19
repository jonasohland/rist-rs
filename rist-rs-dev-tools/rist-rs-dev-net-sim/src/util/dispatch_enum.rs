macro_rules! dispatch_enum {
    
    ($(($name:ident $(,$derived:ty)*) { $($implementation:tt),* }),*) => {
        $(
            #[derive($($derived),*)]
            pub enum $name {
                $($implementation($implementation)),*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            $fname:ident() -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            impl $ftrait for $name {
                $(
                    fn $fname(&self) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname()
                            ),*
                        }
                    }
                )*
            }
        )*
    };


    ($(($name:ident: $ftrait:ty) => {
        $(
            async $fname:ident() -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            #[async_trait]
            impl $ftrait for $name {
                $(
                    async fn $fname(&self) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname()
                            ),*
                        }.await
                    }
                )*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            async $fname:ident(mut self) -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            #[async_trait]
            impl $ftrait for $name {
                $(
                    async fn $fname(mut self) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname()
                            ),*
                        }.await
                    }
                )*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            $fname:ident($arg1_name:ident : $arg1_type:ty) -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            impl $ftrait for $name {
                $(
                    fn $fname(&self, $arg1_name: $arg1_type) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname($arg1_name, $arg2_name)
                            ),*
                        }.await
                    }
                )*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            $fname:ident(&mut self, $arg1_name:ident : $arg1_type:ty) -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            #[async_trait]
            impl $ftrait for $name {
                $(
                    fn $fname(&mut self, $arg1_name: $arg1_type) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname($arg1_name)
                            ),*
                        }
                    }
                )*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            async $fname:ident(&mut self, $arg1_name:ident : $arg1_type:ty) -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            #[async_trait]
            impl $ftrait for $name {
                $(
                    async fn $fname(&mut self, $arg1_name: $arg1_type) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname($arg1_name)
                            ),*
                        }.await
                    }
                )*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            async $fname:ident($arg1_name:ident : $arg1_type:ty, $arg2_name:ident : $arg2_type:ty) -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            #[async_trait]
            impl $ftrait for $name {
                $(
                    async fn $fname(&self, $arg1_name: $arg1_type, $arg2_name: $arg2_type) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname($arg1_name, $arg2_name)
                            ),*
                        }.await
                    }
                )*
            }
        )*
    };

    ($(($name:ident: $ftrait:ty) => {
        $(
            async $fname:ident($arg1_name:ident : $arg1_type:ty, $arg2_name:ident : $arg2_type:ty, $arg3_name:ident : $arg3_type:ty) -> $ret:ty { $($im:tt),* }
        ),*
    }),*) => {
        $(
            #[async_trait]
            impl $ftrait for $name {
                $(
                    async fn $fname(&self, $arg1_name: $arg1_type, $arg2_name: $arg2_type, $arg3_name: $arg3_type) -> $ret {
                        match self {
                            $(
                                $name::$im (o) => o.$fname($arg1_name, $arg2_name, $arg3_name)
                            ),*
                        }.await
                    }
                )*
            }
        )*
    };

}

pub (crate) use dispatch_enum;