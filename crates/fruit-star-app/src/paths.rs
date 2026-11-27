//! 리소스 루트 탐색.

use std::path::{Path, PathBuf};

/// `assets/`와 `config/`가 들어 있는 디렉터리를 찾는다.
///
/// 1. 환경 변수 `FRUIT_STAR_ROOT`
/// 2. 앱 번들의 `Contents/Resources`
/// 3. 실행 파일 위치에서 위로 올라가며 찾은 디렉터리 (개발 빌드: 워크스페이스 루트)
/// 4. 현재 작업 디렉터리
pub fn resource_root() -> PathBuf {
    if let Some(root) = std::env::var_os("FRUIT_STAR_ROOT").map(PathBuf::from)
        && is_root(&root)
    {
        return root;
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(bundle) = exe
            .parent()
            .and_then(Path::parent)
            .map(|contents| contents.join("Resources"))
            && is_root(&bundle)
        {
            return bundle;
        }
        let mut dir = exe.parent();
        while let Some(candidate) = dir {
            if is_root(candidate) {
                return candidate.to_path_buf();
            }
            dir = candidate.parent();
        }
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

fn is_root(dir: &Path) -> bool {
    dir.join("assets").is_dir() && dir.join("config").is_dir()
}
