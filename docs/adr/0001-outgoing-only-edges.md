# Edges are declared outgoing-only, by the caller

A Component's manifest could let both the caller and the callee declare their side of a relationship, but that risks two manifests disagreeing about the same edge over time (one side updated, the other forgotten). We decided that only the caller declares "I call X" in its `diagram.toml`; a Component's incoming edges are always derived by inverting the full graph, never declared directly by the callee.
