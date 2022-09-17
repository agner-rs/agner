pub trait ResultErrFlattenIn {
    type Ok;
    type Err;

    fn err_flatten_in(self) -> Result<Self::Ok, Self::Err>;
}

impl<Ok, EInner, EOuter> ResultErrFlattenIn for Result<Result<Ok, EInner>, EOuter>
where
    EOuter: Into<EInner>,
{
    type Ok = Ok;
    type Err = EInner;

    fn err_flatten_in(
        self,
    ) -> Result<<Self as ResultErrFlattenIn>::Ok, <Self as ResultErrFlattenIn>::Err> {
        match self {
            Ok(Ok(ok)) => Ok(ok),
            Err(e_outer) => Err(e_outer.into()),
            Ok(Err(e_inner)) => Err(e_inner),
        }
    }
}

pub trait ResultErrFlattenOut {
    type Ok;
    type Err;

    fn err_flatten_out(self) -> Result<Self::Ok, Self::Err>;
}

impl<Ok, EInner, EOuter> ResultErrFlattenOut for Result<Result<Ok, EInner>, EOuter>
where
    EInner: Into<EOuter>,
{
    type Ok = Ok;
    type Err = EOuter;

    fn err_flatten_out(
        self,
    ) -> Result<<Self as ResultErrFlattenOut>::Ok, <Self as ResultErrFlattenOut>::Err> {
        match self {
            Ok(Ok(ok)) => Ok(ok),
            Err(e_outer) => Err(e_outer),
            Ok(Err(e_inner)) => Err(e_inner.into()),
        }
    }
}
