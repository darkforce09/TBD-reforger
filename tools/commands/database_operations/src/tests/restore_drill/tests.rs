use super::mig_ver;
use std::path::Path;

#[test]
fn mig_ver_strips_leading_zeros() {
    assert_eq!(mig_ver(Path::new("0001_init.sql")), "1");
    assert_eq!(mig_ver(Path::new("0021_later.sql")), "21");
    assert_eq!(mig_ver(Path::new("10_ten.sql")), "10");
}
