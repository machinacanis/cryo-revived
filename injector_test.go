package cryo

import (
	"errors"
	"reflect"
	"testing"
)

type dA struct{ v string }
type dB struct{ a *dA }
type dC struct{ b *dB }

func TestAutoChain(t *testing.T) {
	in := newInjector()
	// out-of-order registration must still resolve
	_ = in.provide(func(b *dB) *dC { return &dC{b} })
	_ = in.provide(func(a *dA) *dB { return &dB{a} })
	_ = in.provide(func() *dA { return &dA{"hi"} })
	v, err := in.resolve(reflect.TypeOf(&dC{}))
	if err != nil {
		t.Fatal(err)
	}
	if got := v.Interface().(*dC).b.a.v; got != "hi" {
		t.Fatalf("resolved %q", got)
	}
	// second resolve returns same instance
	v2, _ := in.resolve(reflect.TypeOf(&dC{}))
	if v2.Interface().(*dC) != v.Interface().(*dC) {
		t.Fatal("not singleton")
	}
}

func TestCtorError(t *testing.T) {
	in := newInjector()
	want := errors.New("boom")
	_ = in.provide(func() (*dA, error) { return nil, want })
	if _, err := in.resolve(reflect.TypeOf(&dA{})); !errors.Is(err, want) {
		t.Fatalf("expected %v, got %v", want, err)
	}
}

func TestMissingDep(t *testing.T) {
	in := newInjector()
	_ = in.provide(func(a *dA) *dB { return &dB{a} })
	if _, err := in.resolve(reflect.TypeOf(&dB{})); err == nil {
		t.Fatal("expected missing dep error")
	}
}

type x1 struct{ y *x2 }
type x2 struct{ z *x1 }

func TestCycle(t *testing.T) {
	in := newInjector()
	_ = in.provide(func(y *x2) *x1 { return &x1{y} })
	_ = in.provide(func(z *x1) *x2 { return &x2{z} })
	if _, err := in.resolve(reflect.TypeOf(&x1{})); err == nil {
		t.Fatal("expected cycle error")
	} else {
		t.Log(err)
	}
}

func TestInstance(t *testing.T) {
	in := newInjector()
	a := &dA{"direct"}
	_ = in.provide(a)
	v, err := in.resolve(reflect.TypeOf(&dA{}))
	if err != nil || v.Interface().(*dA) != a {
		t.Fatal("instance not returned")
	}
}
