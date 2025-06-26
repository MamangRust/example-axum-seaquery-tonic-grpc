use sea_query::Iden;

#[derive(Debug, Iden)]
pub enum Posts {
    Table,
    Id,
    Title,
    Img,
    Body,
    CategoryId,
    UserId,
    UserName,
}
