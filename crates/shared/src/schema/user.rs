use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Users {
    Table,
    Id,
    Firstname,
    Lastname,
    Email,
    Password,
}
