# `jotta`

A high-level abstraction of the Jottacloud API.

## Architecture

```txt
Jotta/Archive/<ROOT>
├── notes
│   ├── list.txt
│   └── donkey.jpeg
└── videos
    ├── train.mp4
    ├── train.mp4.1
    ├── train.mp4.2
    ├── shrek.mp4
    └── shrek.mp4.1
```

`notes/list.txt`:

```txt
{
  "crc32": "YWm1nw",
  "size": 34
}
my favorite movies:
- shrek
- cars
```

In order to allow relatively painless streaming, larger files are automatically chunked on the filesystem.
