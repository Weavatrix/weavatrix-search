# Node.js and Bun benchmark snapshot

This file is generated. Every number below was produced by the
[weavatrix-benchmarks](https://github.com/Weavatrix/weavatrix-benchmarks)
harness and copied out of its recorded run; none of it is typed by hand.
That repository states the rules every suite obeys, including what each
row had to prove equal before it was allowed to be timed.

**Question.** How fast is finding every matching line in a repository, cold and then repeatedly?

**Competitor.** `fdir + node:fs`

| Property | Value |
| --- | --- |
| Measured | 2026-08-24 |
| Platform | win32 x64, 10.0.26200 |
| CPU | Intel(R) Core(TM) Ultra 7 255U (14 logical cores) |
| Memory | 47.5 GiB |
| Rounds | 7 measured, after 2 warm-ups, alternating order, median reported |
| Independent runs | 3 per suite, each in a fresh process; the table shows the median and the spread |
| Package | weavatrix-search 0.3.2 |

## node 24.15.0

Corpus: `[{"files":2000,"lines":200,"bytes":15920400,"matches":80}]`

| Contract | Parity | Weavatrix | Competitor | Result |
| --- | --- | ---: | ---: | ---: |
| cold repository search | identical sorted {path, lineNumber, line} list | 108.643 ms | 510.148 ms | Weavatrix 4.66x faster (4.59x–4.82x) |
| repeat query over an unchanged corpus through a persistent index | identical sorted {path, lineNumber, line} list | 1.782 ms | 459.653 ms | Weavatrix 253.77x faster (245.15x–290.81x) |

## bun 1.3.14

Corpus: `[{"files":2000,"lines":200,"bytes":15920400,"matches":80}]`

| Contract | Parity | Weavatrix | Competitor | Result |
| --- | --- | ---: | ---: | ---: |
| cold repository search | identical sorted {path, lineNumber, line} list | 114.175 ms | 535.544 ms | Weavatrix 4.75x faster (4.02x–5.00x) |
| repeat query over an unchanged corpus through a persistent index | identical sorted {path, lineNumber, line} list | 2.093 ms | 487.818 ms | Weavatrix 233.07x faster (220.53x–300.95x) |

## Reading these rows

- This is not a comparison against ripgrep. ripgrep is not an npm library; the row measures what a Node or Bun consumer would otherwise write.
- **cold repository search** — Weavatrix also applies ignore rules, binary detection, and encoding handling that the baseline skips
- **repeat query over an unchanged corpus through a persistent index** — the case an editor or agent actually hits; the baseline re-reads the whole corpus every time

## Reproduce

```console
git clone https://github.com/Weavatrix/weavatrix-benchmarks
cd weavatrix-benchmarks && npm ci
node run.mjs --suite=search
bun run.mjs --suite=search
node export.mjs
```

CPU, memory bandwidth, filesystem, antivirus, and JavaScript engine
version all move these timings. Treat them as a reproducible snapshot of
the environment above, not as a universal result.
