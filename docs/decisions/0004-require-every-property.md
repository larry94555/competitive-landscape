# ADR 0004 — Require every property in a schema used to constrain a model

**Status:** accepted
**Date:** 2026-08-03

## Context

[ADR 0003](0003-bound-integer-schemas.md) fixed the first gap between `schemars` and
llama.cpp's grammar converter. This is the second, found the same way — by measuring
something and not believing the result.

The first golden-set run scored Qwen3-1.7B at 53% of fields correct, with `billing_period`
recorded as *missing* on 7 of 10 subjects and `plan_name` on 4. A model that cannot read
"per month" off a pricing table is not a model worth benchmarking, so the harness was the
first suspect. It was right to be.

Sending the generated schema to `llama-server` by hand returned this, on a page that states
both a price and a period:

```json
{"plan_name": "Grower", "price_usd": 49}
```

Two keys, then the closing brace. Not `null` — *absent*. Both Qwen3-1.7B and Qwen3-4B did
it, on the same prompt, every time.

The cause is that `required` in JSON Schema is a **validation** concept. It lists the keys a
document must contain to be accepted; a key not listed may simply be left out. `schemars`
therefore, and correctly, omits `Option<T>` fields from `required` — an absent field and a
`null` field both deserialise to `None`, so for validation they are interchangeable.

For *generation* they are not interchangeable at all. The grammar is built from the schema,
so "may be omitted" becomes a legal path through the grammar, and a model takes it: closing
the object is always available and always cheap. And because `serde` maps a missing key to
`None`, the truncation is invisible after parsing.

That is the part that made this worth an ADR rather than a commit. The failure does not look
like a failure. It looks like the honest abstention the whole product is built to produce:

| What the model did | What the parsed struct says | What we would have concluded |
|---|---|---|
| Read the page, found no price | `price_usd: None` | The page publishes no price ✔ |
| Stopped writing after two keys | `price_usd: None` | The page publishes no price ✘ |

`landscape-core::extract` makes every field `Option` precisely so that a gap is
representable. Left uncorrected, this turns that design against itself: the shape built to
report gaps honestly becomes the shape that hides truncation.

## Decision

`landscape-llm::generate` walks the generated schema and adds every declared property to its
object's `required` list, recursively, before sending it. This sits beside
`tighten_integer_bounds` and runs on the same path, so no report type has to remember an
annotation.

`required` governs the presence of the key, not the value behind it. `Option<T>` still
serialises as `["string","null"]`, so `null` remains legal. The change does not force the
model to produce a value; it forces it to produce a **decision** — write the key, then
choose between a value and `null` — rather than declining to mention the field.

## Consequences

Measured on the golden set, prompt v1, same seed, same subjects:

| | fields correct | perfect subjects |
|---|---|---|
| Before | 53% | 0 / 10 |
| After | 73% | 5 / 10 |

Nothing about the model changed. The 20 points were being thrown away by the harness.

**We now depend on llama.cpp honouring `required` in its converter.** It does today, on
every property including those behind `anyOf` and `$ref`, verified by hand against both
models. The signal that this has broken is the same one that exposed it: a field that is
never populated on subjects where the golden set says it should be.

**A schema sent for generation is not the same artifact as one sent for validation**, and
this is now the second time that has cost us a day. Anything we publish for third parties to
validate against must be the `schemars` output, not the constrained one — a schema that
demands every key be present would reject documents our own API returns.

## What this does not fix

Requiring the key makes the model answer. It does not make the answer right, and two related
things were measured and deliberately left alone:

**Field order is a real lever and we are not pulling it yet.** The grammar walks properties
in the order the schema serialises them, which is alphabetical, because `serde_json` maps are
sorted. llama.cpp respects the order it is given — verified by sending a hand-ordered schema
— so `serde_json`'s `preserve_order` feature would hand us control. Whether the evidence
quote should come first (read, then answer) or last (answer, then justify) is worth
measuring, and a hand probe showed quote-first spends the entire token budget quoting. That
is its own change, with its own numbers.

**Making the invalid state unrepresentable measured worse.** Both good models returned
`billing_period: "monthly"` for plans they had correctly reported as having no price. The
obvious fix is a tagged union — a period cannot exist without an amount — and llama.cpp
handles the `oneOf` cleanly. But on a page plainly reading `$49 per month`, Qwen3-4B chose
the `"not_published"` branch. Constrained decoding does not make branches equally likely: a
one-token escape hatch competes with a nested object, and the cheap branch wins. The
invariant moved into the prompt instead (v2), where it cost nothing.

That second result is the general lesson, and it is not obvious: **a type that makes a bad
answer impossible can make the good answer less likely.** Type-level guarantees are free
when a compiler enforces them and are emphatically not free when a sampler does.
