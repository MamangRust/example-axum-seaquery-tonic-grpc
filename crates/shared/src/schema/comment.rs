use sea_query::Iden;



#[derive(Debug, Iden)]
pub enum Comments {
    Table,
    Id,
    IdPostComment,
    UserNameComment,
    Comment,
}