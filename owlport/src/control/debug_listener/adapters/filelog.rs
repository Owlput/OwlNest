pub struct FilelogAdapter{
    path:String
}
impl FilelogAdapter{
    pub fn new(path:String)->Self{
        FilelogAdapter{
            path
        }
    }
}