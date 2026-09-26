import fc from "fast-check";

export interface EdgeModel {
  target: { kind: "component" | "external"; index: number };
  via: string | null;
  data: string | null;
  fromPart: number | null;
  toPart: number | null;
}

export interface PartModel {
  name: string;
}

export interface PartEdgeModel {
  from: number;
  to: number;
  via: string | null;
}

export interface ComponentModel {
  name: string;
  environment: number | null;
  parts: PartModel[];
  edges: EdgeModel[];
  partEdges: PartEdgeModel[];
}

export interface ViewportSize {
  width: number;
  height: number;
  animate: boolean;
}

export interface Model {
  environments: string[];
  components: ComponentModel[];
  externals: string[];
}

const ASCII_CHARS = "abcdefghijklmnopqrstuvwxyz0123456789-_.";
const UNICODE_PIECES = [
  "é", "ü", "ñ", "ø", "ß", "é", "ä",
  "中", "文", "字", "界", "日本", "한국",
  "😀", "🚀", "👩‍💻", "🇫🇷", "👍🏽",
  "שלום", "مرحبا",
  "ｗｉｄｅ", "Ω", "∑",
];

const asciiChar = fc.constantFrom(...ASCII_CHARS.split(""));

const asciiName = fc.oneof(
  { weight: 3, arbitrary: fc.array(asciiChar, { minLength: 1, maxLength: 24 }).map((c) => c.join("")) },
  { weight: 1, arbitrary: fc.array(asciiChar, { minLength: 25, maxLength: 120 }).map((c) => c.join("")) }
);

const unicodeName = fc
  .array(fc.oneof({ weight: 2, arbitrary: asciiChar }, { weight: 1, arbitrary: fc.constantFrom(...UNICODE_PIECES) }), {
    minLength: 1,
    maxLength: 60,
  })
  .map((c) => c.join(""));

export const nameArb = fc.oneof({ weight: 4, arbitrary: asciiName }, { weight: 1, arbitrary: unicodeName });

const labelArb = fc.option(nameArb, { nil: null, freq: 3 });
const indexArb = fc.nat({ max: 63 });

const edgeArb: fc.Arbitrary<EdgeModel> = fc.record({
  target: fc.record({
    kind: fc.constantFrom<"component" | "external">("component", "component", "external"),
    index: indexArb,
  }),
  via: labelArb,
  data: labelArb,
  fromPart: fc.option(indexArb, { nil: null }),
  toPart: fc.option(indexArb, { nil: null }),
});

const componentArb: fc.Arbitrary<ComponentModel> = fc.record({
  name: nameArb,
  environment: fc.option(indexArb, { nil: null, freq: 5 }),
  parts: fc.uniqueArray(nameArb, { maxLength: 8 }).map((names) => names.map((n) => ({ name: n }))),
  edges: fc.array(edgeArb, { maxLength: 4 }),
  partEdges: fc.array(fc.record({ from: indexArb, to: indexArb, via: labelArb }), { maxLength: 3 }),
});

export const modelArb: fc.Arbitrary<Model> = fc.record({
  environments: fc.uniqueArray(nameArb, { maxLength: 4 }),
  components: fc.uniqueArray(componentArb, { minLength: 1, maxLength: 12, selector: (c) => c.name }),
  externals: fc.uniqueArray(nameArb, { maxLength: 3 }),
});
