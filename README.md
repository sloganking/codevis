# rs-code-visualizer

**rs-code-visualizer** takes all source code / UTF-8 encodable text that you put in the ``./input/`` folder, and renders them to one PNG image. These images show the shape and size of files, but not the exact characters inside the files. If you add non UTF-8 compatible files, they will be skipped from the render. It syntax highlights any type of file it understands.

This repo's render of [sloganking/My-Own-OS](https://github.com/sloganking/My-Own-OS/tree/6e555c05ce46dcc13904eb41cc4b3ccde61032b5):
 
![](./assets/code.png)


## Usage

- Install [the Rust programming language.](https://www.rust-lang.org/)
- Put all files you want to visualize in the ``./input/`` folder (they can be in deeper folders).
- Open a terminal in this repo's directory, and run ``cargo run --release``
- See the finished render in ``./output.png``


### To document

- How to set the target aspect ratio
- How to change themes
