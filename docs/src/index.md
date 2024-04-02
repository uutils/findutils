<!-- markdownlint-disable MD041 -->

{{#include logo.svg}}

<!-- markdownlint-disable MD033 -->

<style>
    /* Make the logo a bit bigger and center */
    #logo {
        height: 200px;
        width: 100%;
    }

    /* This is necessary to get the <use> tags to obey the CSS styles below */
    g, polygon {
        fill: inherit;
        stroke: inherit;
    }

    /* Set the circle to the foreground color of the theme */
    #gear circle {
        stroke: var(--fg);
    }

    /* Set the stroke of polygons and the copies (via use) */
    #gear polygon,
    #gear use {
        fill: var(--fg);
        stroke: var(--fg);
    }
</style>

# uutils findutils Documentation

The uutils findutils project reimplements the GNU findutils in Rust. It is available for Linux, Windows, Mac
and other platforms.

uutils is licensed under the
[MIT License](https://github.com/uutils/findutils/blob/main/LICENSE).

## Useful links

- [Releases](https://github.com/uutils/findutils/releases)
- [Source Code](https://github.com/uutils/findutils)
- [Issues](https://github.com/uutils/findutils/issues)
- [Discord](https://discord.gg/wQVJbvJ)

> Note: This manual is automatically generated from the source code and is a
> work in progress.
