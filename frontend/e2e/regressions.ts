import type { Action } from "./actions";
import type { Model, ViewportSize } from "./model";

export const regressions: [Model, ViewportSize, Action[]][] = [
  // resized component escaped its environment
  [{"environments": [], "components": [{"name": "aaaaaaaaaaaaaaaaaaaaaaaaa", "environment": null, "parts": [], "edges": [], "partEdges": []}], "externals": []}, {"width": 1400, "height": 900, "animate": false}, [{"kind": "resize", "node": "component", "index": 0, "dx": 95, "dy": 0}]],
  // resized part escaped its component's interior
  [{"environments": [], "components": [{"name": "a", "environment": null, "parts": [{"name": "p"}], "edges": [], "partEdges": []}], "externals": []}, {"width": 1400, "height": 900, "animate": false}, [{"kind": "resize", "node": "part", "index": 0, "dx": 150, "dy": 150}]],
];
