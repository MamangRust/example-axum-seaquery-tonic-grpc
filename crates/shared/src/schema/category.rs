use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Categories {
    Table,
    Id,
    Name,
}
