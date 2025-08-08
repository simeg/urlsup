#!/usr/bin/env python3

"""
Simple script to generate test directory structure for urlsup testing.
Creates test-links-dir/dir-one/dir-two/ with three .md files containing URLs.

Uses only Python 3 standard library - no external dependencies required.
"""

import os
from pathlib import Path


def main():
    """Create test directory structure and generate three .md files"""
    
    print("🔗 Creating test directory structure...")
    
    # Create the base directory
    base_dir = Path("test-links-dir")
    base_dir.mkdir(exist_ok=True)
    
    # Create three different directories
    dir1 = base_dir
    dir2 = base_dir / "dir-one"
    dir3 = base_dir / "dir-one" / "dir-two"
    
    dir2.mkdir(exist_ok=True)
    dir3.mkdir(exist_ok=True)
    
    # File 1: Working URLs (in test-links-dir/dir-one/)
    working_urls = """# Working URLs Test File

This file contains URLs that should work:

- GitHub: https://github.com
- Example: https://example.com
- HTTPBin: https://httpbin.org/get
- Google: https://google.com
- Rust docs: https://doc.rust-lang.org/
- Crates.io: https://crates.io/
"""
    
    with open(dir2 / "working-urls.md", "w") as f:
        f.write(working_urls)
    
    # File 2: Broken URLs (in test-links-dir/)
    broken_urls = """# Broken URLs Test File

This file contains URLs that should fail:

- Non-existent domain: https://this-domain-does-not-exist-12345.invalid
- 404 error: https://httpbin.org/status/404
- 500 error: https://httpbin.org/status/500
- Timeout: https://httpbin.org/delay/60
- Invalid URL: https://
"""
    
    with open(dir1 / "broken-urls.md", "w") as f:
        f.write(broken_urls)
    
    # File 3: Mixed URLs (in test-links-dir/dir-one/dir-two/)
    mixed_urls = """# Mixed URLs Test File

This file contains a mix of working and broken URLs:

- Working: https://example.com
- Broken: https://fake-domain-12345.com
- Working: https://httpbin.org/status/200
- Broken: https://httpbin.org/status/404
- Working: https://github.com/microsoft/vscode
- Broken: https://github.com/non-existent-user/non-existent-repo
"""
    
    with open(dir3 / "mixed-urls.md", "w") as f:
        f.write(mixed_urls)
    
    # Update .gitignore
    gitignore_path = Path(".gitignore")
    if gitignore_path.exists():
        with open(gitignore_path, "r") as f:
            content = f.read()
        
        if "test-links-dir" not in content:
            with open(gitignore_path, "a") as f:
                f.write("\n# Generated test directory\ntest-links-dir/\n")
            print("✅ Added test-links-dir/ to .gitignore")
        else:
            print("ℹ️  test-links-dir/ already in .gitignore")
    else:
        with open(gitignore_path, "w") as f:
            f.write("# Generated test directory\ntest-links-dir/\n")
        print("✅ Created .gitignore with test-links-dir/ entry")
    
    print("✅ Created test-links-dir/ with 3 .md files in different directories")
    print()
    print("💡 Usage:")
    print("   ./urlsup test-links-dir/ --recursive")
    print("   ./urlsup test-links-dir/dir-one/dir-two/")


if __name__ == "__main__":
    main()