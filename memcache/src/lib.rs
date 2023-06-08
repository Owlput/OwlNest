pub mod manager;
pub mod store;

#[cfg(test)]
mod tests {

    use crate::manager::{MapManager, MapResult::*};

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn it_works() {
        let manager = MapManager::new(8);
        let manager1 = manager.clone();
        assert_eq!(manager.map_try_insert("ping".to_string(),"PONG".to_string()).await,Inserted);
        assert_eq!(manager1.map_try_insert("ping".to_string(), "PONG".to_string()).await,Failed);
        assert_eq!(manager1.map_try_insert("123".to_string(), "abc".to_string()).await,Inserted);
        assert_eq!(manager.map_get_single("123".to_string()).await,FoundValue(Some("abc".to_string())));
        assert_eq!(manager.map_set("123".to_string(), "def".to_string()).await,Modified("abc".to_string()));
        assert_eq!(manager.map_get_single("123".to_string()).await,FoundValue(Some("def".to_string())));
        assert_eq!(manager1.map_delete("ping".to_string()).await,Deleted("PONG".to_string()));
    }
}
