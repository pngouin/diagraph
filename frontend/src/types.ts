export type NodeKind = "component" | "external" | "environment" | "part";

export interface JsonNode {
  id: string;
  label: string;
  kind: NodeKind;
  environment: string | null;
}

export interface JsonEdge {
  from: string;
  to: string;
  label: string | null;
  crossEnvironment: boolean;
}

export interface JsonView {
  nodes: JsonNode[];
  edges: JsonEdge[];
}

export interface Payload {
  environments: string[];
  views: {
    global: JsonView;
    environment: JsonView;
    zoomed: Record<string, JsonView>;
  };
}
