pub enum DataType {
    RMC,
    GGA,
    VTG,
}

pub struct NmeaData {
    d_type: DataType,
    data: DataType,
}

impl NmeaData {
    pub fn new(d_type: DataType, data: DataType) -> NmeaData {
        NmeaData { d_type, data }
    }
}
