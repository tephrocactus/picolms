use arrow::array::ArrayBuilder;
use arrow::array::BooleanBuilder;
use arrow::array::Float64Builder;
use arrow::array::Int64Builder;
use arrow::array::ListBuilder;
use arrow::array::StringBuilder;
use arrow::datatypes::DataType;
use arrow::datatypes::Field;

pub trait DomainField {
    fn builder(&self) -> Box<dyn ArrayBuilder>;
    fn append_null(&self, builder: &mut dyn ArrayBuilder);
}

impl DomainField for Field {
    fn builder(&self) -> Box<dyn ArrayBuilder> {
        match self.data_type() {
            DataType::Utf8 => Box::new(StringBuilder::new()),
            DataType::Int64 => Box::new(Int64Builder::new()),
            DataType::Float64 => Box::new(Float64Builder::new()),
            DataType::Boolean => Box::new(BooleanBuilder::new()),
            DataType::List(v) => match v.data_type() {
                DataType::Utf8 => Box::new(ListBuilder::new(StringBuilder::new())),
                DataType::Int64 => Box::new(ListBuilder::new(Int64Builder::new())),
                DataType::Float64 => Box::new(ListBuilder::new(Float64Builder::new())),
                DataType::Boolean => Box::new(ListBuilder::new(BooleanBuilder::new())),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    fn append_null(&self, builder: &mut dyn ArrayBuilder) {
        let b = builder.as_any_mut();
        match self.data_type() {
            DataType::Utf8 => b.downcast_mut::<StringBuilder>().unwrap().append_null(),
            DataType::Int64 => b.downcast_mut::<Int64Builder>().unwrap().append_null(),
            DataType::Float64 => b.downcast_mut::<Float64Builder>().unwrap().append_null(),
            DataType::Boolean => b.downcast_mut::<BooleanBuilder>().unwrap().append_null(),
            DataType::List(v) => match v.data_type() {
                DataType::Utf8 => b
                    .downcast_mut::<ListBuilder<StringBuilder>>()
                    .unwrap()
                    .append_null(),
                DataType::Int64 => b
                    .downcast_mut::<ListBuilder<Int64Builder>>()
                    .unwrap()
                    .append_null(),
                DataType::Float64 => b
                    .downcast_mut::<ListBuilder<Float64Builder>>()
                    .unwrap()
                    .append_null(),
                DataType::Boolean => b
                    .downcast_mut::<ListBuilder<BooleanBuilder>>()
                    .unwrap()
                    .append_null(),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}
