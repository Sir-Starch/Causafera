# Phonology

Phonology defines the sound systems of simulated languages. Each language community may have distinct phoneme inventories, syllable structures, stress patterns, and phonotactic constraints.

## Phoneme Inventory

A language defines its allowed phonemes:

```text
consonants: t, k, m, n, r, s, v, ...
vowels: a, e, i, o, ...
```

These are not IPA symbols stored in simulation state. They are abstract phonological categories that constrain possible utterances.

## Syllable Structure

Languages define syllable templates:

```text
allowed onsets: t, k, m, n, r, s, v, tr, kr
nucleus: vowel
allowed codas: n, r, s, m
```

Candidate output respecting these constraints: `/tren/`

## Phonotactics

Beyond syllable structure, languages may have constraints on:

- Consonant clusters across morpheme boundaries
- Vowel harmony or disharmony
- Tone or pitch accent systems
- Stress assignment rules
- Phonological processes (assimilation, dissimilation, lenition, fortition)

## Novel Word Generation

When an agent coins a new lexeme, the form must respect the phonological constraints of the agent's language. Generation is deterministic in strict mode, with possible key inputs:

```text
world_seed
language_id
speaker_id
concept_id
coinage_event_id
```

A phonotactic generator produces a novel form that satisfies the language's constraints. Only the originating speaker initially possesses the association between this form and their concept.

## Sound Change

Over time, phonological forms may shift. The architecture must support future implementation of:

- Regular sound change (conditioned phonetic shifts)
- Phonetic drift (gradual articulatory shifts)
- Merger (distinct phonemes collapsing)
- Split (single phoneme diverging in context)
- Chain shifts (push chains, drag chains)

Language evolution operates at community or cohort resolution, not by simulating every acoustic event.

## Acoustic Properties and Mana

Physical utterances have measurable acoustic properties: frequency, timing, repetition, synchronization. Mana does not understand words, but it may respond to these physical patterns. Therefore, phonological change can alter magical practices without any change in semantic intent.

## Related Documents

- `docs/language/morphology.md` - How phonological forms combine into larger structures
- `docs/language/language-change.md` - Sound change as a mechanism of language evolution
- `docs/language/lexical-innovation.md` - How new phonological forms are generated
