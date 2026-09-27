import type { Action } from "./actions";
import type { Model, ViewportSize } from "./model";

export const regressions: [Model, ViewportSize, Action[]][] = [
  // resized component escaped its environment
  [{"environments": [], "components": [{"name": "aaaaaaaaaaaaaaaaaaaaaaaaa", "environment": null, "parts": [], "edges": [], "partEdges": []}], "externals": []}, {"width": 1400, "height": 900, "animate": false}, [{"kind": "resize", "node": "component", "index": 0, "dx": 95, "dy": 0}]],
  // resized part escaped its component's interior
  [{"environments": [], "components": [{"name": "a", "environment": null, "parts": [{"name": "p"}], "edges": [], "partEdges": []}], "externals": []}, {"width": 1400, "height": 900, "animate": false}, [{"kind": "resize", "node": "part", "index": 0, "dx": 150, "dy": 150}]],
  // mid-zoom ambient fade greyed out unselected components
  [{"environments": ["e"], "components": [{"name": "svc0", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc1", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc2", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc3", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc4", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc5", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc6", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc7", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc8", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc9", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc10", "environment": 0, "parts": [], "edges": [], "partEdges": []}, {"name": "svc11", "environment": 0, "parts": [], "edges": [], "partEdges": []}], "externals": []}, {"width": 1400, "height": 900, "animate": false}, [{"kind": "zoomAt", "node": "component", "index": 0, "deltaY": -982}]],
  // dimming a selected part's siblings showed part-internal edges its zoom reveal kept hidden
  [{"environments": [], "components": [{"name": "svc", "environment": null, "parts": [{"name": "p0"}, {"name": "p1"}, {"name": "p2"}], "edges": [], "partEdges": [{"from": 1, "to": 2, "via": null}]}], "externals": []}, {"width": 1400, "height": 900, "animate": false}, [{"kind": "zoomAt", "node": "component", "index": 0, "deltaY": -1500}, {"kind": "select", "node": "part", "index": 0}, {"kind": "zoomAt", "node": "component", "index": 0, "deltaY": 1500}]],
];
