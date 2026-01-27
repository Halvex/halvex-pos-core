use serde::{Deserialize, Serialize};
use uuid::Uuid;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(pub Uuid);

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }
    };
}

id_newtype!(VenueID);
id_newtype!(OrderID);
id_newtype!(OrderItemID);
id_newtype!(MenuItemID);
id_newtype!(ModifierID);
id_newtype!(UserID);
id_newtype!(ActorID);
id_newtype!(TableID);
id_newtype!(DeviceID);
id_newtype!(CheckID);
id_newtype!(AreaID);
id_newtype!(PaymentID);
