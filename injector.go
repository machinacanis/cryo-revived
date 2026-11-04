package cryo

import (
	"fmt"
	"reflect"
	"strings"
)

// injector is the dependency container: it stores ready instances and
// constructor functions, then resolves a type by walking the constructor
// graph automatically (memoized singletons, cycle detection).
type injector struct {
	instances map[reflect.Type]reflect.Value
	ctors     map[reflect.Type]reflect.Value
}

var errType = reflect.TypeOf((*error)(nil)).Elem()

func newInjector() *injector {
	return &injector{
		instances: map[reflect.Type]reflect.Value{},
		ctors:     map[reflect.Type]reflect.Value{},
	}
}

// provide registers either a ready-made instance (*T) or a constructor:
//
//	func(deps...) *T
//	func(deps...) (*T, error)
//
// Constructor parameters are themselves resolved from the graph, so
// registration order does not matter.
func (i *injector) provide(v interface{}) error {
	rv := reflect.ValueOf(v)
	if rv.Kind() != reflect.Func {
		i.instances[rv.Type()] = rv
		return nil
	}
	t := rv.Type()
	if t.NumOut() != 1 && t.NumOut() != 2 {
		return fmt.Errorf("cryo: constructor %s must return T or (T, error)", t)
	}
	if t.NumOut() == 2 && !t.Out(1).Implements(errType) {
		return fmt.Errorf("cryo: constructor %s second return must be error", t)
	}
	i.ctors[t.Out(0)] = rv
	return nil
}

// resolve returns the (singleton) value for type t, building it through
// constructors on first request.
func (i *injector) resolve(t reflect.Type) (reflect.Value, error) {
	return i.walk(t, nil)
}

func (i *injector) walk(t reflect.Type, chain []reflect.Type) (reflect.Value, error) {
	if v, ok := i.instances[t]; ok {
		return v, nil
	}
	for _, c := range chain {
		if c == t {
			path := make([]string, 0, len(chain)+1)
			for _, p := range chain {
				path = append(path, p.String())
			}
			return reflect.Value{}, fmt.Errorf("cryo: dependency cycle: %s",
				strings.Join(append(path, t.String()), " -> "))
		}
	}
	ctor, ok := i.ctors[t]
	if !ok {
		return reflect.Value{}, fmt.Errorf("cryo: no service or constructor for %s", t)
	}
	ct := ctor.Type()
	args := make([]reflect.Value, ct.NumIn())
	for j := range args {
		v, err := i.walk(ct.In(j), append(chain, t))
		if err != nil {
			return reflect.Value{}, err
		}
		args[j] = v
	}
	outs := ctor.Call(args)
	if ct.NumOut() == 2 {
		if e := outs[1].Interface(); e != nil {
			return reflect.Value{}, fmt.Errorf("cryo: constructor for %s failed: %w", t, e.(error))
		}
	}
	v := outs[0]
	i.instances[t] = v
	return v, nil
}

// warm eagerly resolves every registered constructor so missing deps and
// cycles fail at boot instead of mid-event.
func (i *injector) warm() error {
	for t := range i.ctors {
		if _, err := i.resolve(t); err != nil {
			return err
		}
	}
	return nil
}
