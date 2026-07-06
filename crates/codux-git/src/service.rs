pub struct GitService;

include!("service/repository.rs");
include!("service/review.rs");
include!("service/commit.rs");
include!("service/remote.rs");
include!("service/branch.rs");
include!("service/stash.rs");
include!("service/tag.rs");
include!("service/gitignore.rs");
