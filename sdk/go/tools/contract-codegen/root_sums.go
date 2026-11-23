package main

// exposeRootObjectSum supplies the redundant object type that go-jsonschema
// requires before generating a root declaration. Every admitted branch already
// requires an object; the canonical schema and closed-union inventory stay intact.
func exposeRootObjectSum(value any) {
	root, ok := value.(map[string]any)
	if !ok {
		return
	}
	if _, typed := root["type"]; typed {
		return
	}
	if _, shaped := root["properties"]; shaped {
		return
	}
	branches, ok := root["anyOf"].([]any)
	if !ok || len(branches) < 2 {
		return
	}
	for _, value := range branches {
		branch, ok := value.(map[string]any)
		if !ok || branch["type"] != "object" {
			return
		}
	}
	root["type"] = "object"
}
