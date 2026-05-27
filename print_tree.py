import os

def print_tree(root=".", prefix="", ignore=None):
    if ignore is None:
        ignore = {".git", "__pycache__", ".DS_Store", "node_modules", ".venv", "target", "dist"}

    entries = sorted(
        [e for e in os.scandir(root) if e.name not in ignore],
        key=lambda e: (not e.is_dir(), e.name.lower()),
    )

    for i, entry in enumerate(entries):
        is_last = i == len(entries) - 1
        connector = "└── " if is_last else "├── "
        print(prefix + connector + entry.name)

        if entry.is_dir():
            extension = "    " if is_last else "│   "
            print_tree(entry.path, prefix + extension, ignore)


if __name__ == "__main__":
    print(".")
    print_tree()
