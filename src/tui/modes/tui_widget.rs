pub trait TuiWidget {
    fn widget<RetType>(&self) -> RetType;
}
