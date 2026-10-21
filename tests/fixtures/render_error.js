(() => {
  var __create = Object.create;
  var __getProtoOf = Object.getPrototypeOf;
  var __defProp = Object.defineProperty;
  var __getOwnPropNames = Object.getOwnPropertyNames;
  var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
  var __hasOwnProp = Object.prototype.hasOwnProperty;
  function __accessProp(key) {
    return this[key];
  }
  var __toESMCache_node;
  var __toESMCache_esm;
  var __toESM = (mod, isNodeMode, target) => {
    var canCache = mod != null && typeof mod === "object";
    if (canCache) {
      var cache = isNodeMode ? __toESMCache_node ??= new WeakMap : __toESMCache_esm ??= new WeakMap;
      var cached = cache.get(mod);
      if (cached)
        return cached;
    }
    target = mod != null ? __create(__getProtoOf(mod)) : {};
    const to = isNodeMode || !mod || !mod.__esModule || !__hasOwnProp.call(mod, "default") ? __defProp(target, "default", { value: mod, enumerable: true }) : target;
    if (mod && typeof mod === "object" || typeof mod === "function") {
      for (let key of __getOwnPropNames(mod))
        if (!__hasOwnProp.call(to, key))
          __defProp(to, key, {
            get: __accessProp.bind(mod, key),
            enumerable: true
          });
    }
    if (canCache)
      cache.set(mod, to);
    return to;
  };
  var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
  var __name = (target, name) => {
    Object.defineProperty(target, "name", {
      value: name,
      enumerable: false,
      configurable: true
    });
    return target;
  };
  var __knownSymbol = (name, symbol) => (symbol = Symbol[name]) ? symbol : Symbol.for("Symbol." + name);
  var __typeError = (msg) => {
    throw TypeError(msg);
  };
  var __defNormalProp = (obj, key, value) => (key in obj) ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
  var __accessCheck = (obj, member, msg) => member.has(obj) || __typeError("Cannot " + msg);
  var __privateIn = (member, obj) => Object(obj) !== obj ? __typeError('Cannot use the "in" operator on this value') : member.has(obj);
  var __privateGet = (obj, member, getter) => (__accessCheck(obj, member, "read from private field"), getter ? getter.call(obj) : member.get(obj));
  var __privateSet = (obj, member, value, setter) => (__accessCheck(obj, member, "write to private field"), setter ? setter.call(obj, value) : member.set(obj, value), value);
  var __privateMethod = (obj, member, method) => (__accessCheck(obj, member, "access private method"), method);
  var __decoratorStart = (base) => [, , , __create(base?.[__knownSymbol("metadata")] ?? null)];
  var __decoratorStrings = ["class", "method", "getter", "setter", "accessor", "field", "value", "get", "set"];
  var __expectFn = (fn) => fn !== undefined && typeof fn !== "function" ? __typeError("Function expected") : fn;
  var __decoratorContext = (kind, name, done, metadata, fns) => ({
    kind: __decoratorStrings[kind],
    name,
    metadata,
    addInitializer: (fn) => done._ ? __typeError("Already initialized") : fns.push(__expectFn(fn || null))
  });
  var __decoratorMetadata = (array, target) => __defNormalProp(target, __knownSymbol("metadata"), array[3]);
  var __runInitializers = (array, flags, self, value) => {
    for (var i = 0, fns = array[flags >> 1], n = fns && fns.length;i < n; i++)
      flags & 1 ? fns[i].call(self) : value = fns[i].call(self, value);
    return value;
  };
  var __decorateElement = (array, flags, name, decorators, target, extra) => {
    var fn, it, done, ctx, access, k = flags & 7, s = !!(flags & 8), p = !!(flags & 16);
    var j = k > 3 ? array.length + 1 : k ? s ? 1 : 2 : 0, key = __decoratorStrings[k + 5];
    var initializers = k > 3 && (array[j - 1] = []), extraInitializers = array[j] || (array[j] = []);
    var desc = k && (!p && !s && (target = target.prototype), k < 5 && (k > 3 || !p) && __getOwnPropDesc(k < 4 ? target : {
      get [name]() {
        return __privateGet(this, extra);
      },
      set [name](x) {
        __privateSet(this, extra, x);
      }
    }, name));
    k ? p && k < 4 && __name(extra, (k > 2 ? "set " : k > 1 ? "get " : "") + name) : __name(target, name);
    for (var i = decorators.length - 1;i >= 0; i--) {
      ctx = __decoratorContext(k, name, done = {}, array[3], extraInitializers);
      if (k) {
        ctx.static = s, ctx.private = p, access = ctx.access = { has: p ? (x) => __privateIn(target, x) : (x) => (name in x) };
        if (k ^ 3)
          access.get = p ? (x) => (k ^ 1 ? __privateGet : __privateMethod)(x, target, k ^ 4 ? extra : desc.get) : (x) => x[name];
        if (k > 2)
          access.set = p ? (x, y) => __privateSet(x, target, y, k ^ 4 ? extra : desc.set) : (x, y) => x[name] = y;
      }
      it = (0, decorators[i])(k ? k < 4 ? p ? extra : desc[key] : k > 4 ? undefined : { get: desc.get, set: desc.set } : target, ctx);
      done._ = 1;
      if (k ^ 4 || it === undefined)
        __expectFn(it) && (k > 4 ? initializers.unshift(it) : k ? p ? extra = it : desc[key] = it : target = it);
      else if (typeof it !== "object" || it === null)
        __typeError("Object expected");
      else
        __expectFn(fn = it.get) && (desc.get = fn), __expectFn(fn = it.set) && (desc.set = fn), __expectFn(fn = it.init) && initializers.unshift(fn);
    }
    return k || __decoratorMetadata(array, target), desc && __defProp(target, name, desc), p ? k ^ 4 ? extra : desc : target;
  };

  // ../../node_modules/.bun/react@19.3.1/node_modules/react/cjs/react.production.js
  var require_react_production = __commonJS(function(exports) {
    var REACT_ELEMENT_TYPE = Symbol.for("react.transitional.element");
    var REACT_PORTAL_TYPE = Symbol.for("react.portal");
    var REACT_FRAGMENT_TYPE = Symbol.for("react.fragment");
    var REACT_STRICT_MODE_TYPE = Symbol.for("react.strict_mode");
    var REACT_PROFILER_TYPE = Symbol.for("react.profiler");
    var REACT_CONSUMER_TYPE = Symbol.for("react.consumer");
    var REACT_CONTEXT_TYPE = Symbol.for("react.context");
    var REACT_FORWARD_REF_TYPE = Symbol.for("react.forward_ref");
    var REACT_SUSPENSE_TYPE = Symbol.for("react.suspense");
    var REACT_MEMO_TYPE = Symbol.for("react.memo");
    var REACT_LAZY_TYPE = Symbol.for("react.lazy");
    var REACT_ACTIVITY_TYPE = Symbol.for("react.activity");
    var REACT_VIEW_TRANSITION_TYPE = Symbol.for("react.view_transition");
    var MAYBE_ITERATOR_SYMBOL = Symbol.iterator;
    function getIteratorFn(maybeIterable) {
      if (maybeIterable === null || typeof maybeIterable !== "object")
        return null;
      maybeIterable = MAYBE_ITERATOR_SYMBOL && maybeIterable[MAYBE_ITERATOR_SYMBOL] || maybeIterable["@@iterator"];
      return typeof maybeIterable === "function" ? maybeIterable : null;
    }
    var ReactNoopUpdateQueue = {
      isMounted: function() {
        return false;
      },
      enqueueForceUpdate: function() {},
      enqueueReplaceState: function() {},
      enqueueSetState: function() {}
    };
    var assign = Object.assign;
    var emptyObject = {};
    function Component(props, context, updater) {
      this.props = props;
      this.context = context;
      this.refs = emptyObject;
      this.updater = updater || ReactNoopUpdateQueue;
    }
    Component.prototype.isReactComponent = {};
    Component.prototype.setState = function(partialState, callback) {
      if (typeof partialState !== "object" && typeof partialState !== "function" && partialState != null)
        throw Error("takes an object of state variables to update or a function which returns an object of state variables.");
      this.updater.enqueueSetState(this, partialState, callback, "setState");
    };
    Component.prototype.forceUpdate = function(callback) {
      this.updater.enqueueForceUpdate(this, callback, "forceUpdate");
    };
    function ComponentDummy() {}
    ComponentDummy.prototype = Component.prototype;
    function PureComponent(props, context, updater) {
      this.props = props;
      this.context = context;
      this.refs = emptyObject;
      this.updater = updater || ReactNoopUpdateQueue;
    }
    var pureComponentPrototype = PureComponent.prototype = new ComponentDummy;
    pureComponentPrototype.constructor = PureComponent;
    assign(pureComponentPrototype, Component.prototype);
    pureComponentPrototype.isPureReactComponent = true;
    var isArrayImpl = Array.isArray;
    function noop() {}
    var ReactSharedInternals = { H: null, A: null, T: null, S: null };
    var hasOwnProperty = Object.prototype.hasOwnProperty;
    function ReactElement(type, key, props) {
      var refProp = props.ref;
      return {
        $$typeof: REACT_ELEMENT_TYPE,
        type,
        key,
        ref: refProp !== undefined ? refProp : null,
        props
      };
    }
    function cloneAndReplaceKey(oldElement, newKey) {
      return ReactElement(oldElement.type, newKey, oldElement.props);
    }
    function isValidElement(object) {
      return typeof object === "object" && object !== null && object.$$typeof === REACT_ELEMENT_TYPE;
    }
    function escape(key) {
      var escaperLookup = { "=": "=0", ":": "=2" };
      return "$" + key.replace(/[=:]/g, function(match) {
        return escaperLookup[match];
      });
    }
    var userProvidedKeyEscapeRegex = /\/+/g;
    function getElementKey(element, index) {
      return typeof element === "object" && element !== null && element.key != null ? escape("" + element.key) : index.toString(36);
    }
    function resolveThenable(thenable) {
      switch (thenable.status) {
        case "fulfilled":
          return thenable.value;
        case "rejected":
          throw thenable.reason;
        default:
          switch (typeof thenable.status === "string" ? thenable.then(noop, noop) : (thenable.status = "pending", thenable.then(function(fulfilledValue) {
            thenable.status === "pending" && (thenable.status = "fulfilled", thenable.value = fulfilledValue);
          }, function(error) {
            thenable.status === "pending" && (thenable.status = "rejected", thenable.reason = error);
          })), thenable.status) {
            case "fulfilled":
              return thenable.value;
            case "rejected":
              throw thenable.reason;
          }
      }
      throw thenable;
    }
    function mapIntoArray(children, array, escapedPrefix, nameSoFar, callback) {
      var type = typeof children;
      if (type === "undefined" || type === "boolean")
        children = null;
      var invokeCallback = false;
      if (children === null)
        invokeCallback = true;
      else
        switch (type) {
          case "bigint":
          case "string":
          case "number":
            invokeCallback = true;
            break;
          case "object":
            switch (children.$$typeof) {
              case REACT_ELEMENT_TYPE:
              case REACT_PORTAL_TYPE:
                invokeCallback = true;
                break;
              case REACT_LAZY_TYPE:
                return invokeCallback = children._init, mapIntoArray(invokeCallback(children._payload), array, escapedPrefix, nameSoFar, callback);
            }
        }
      if (invokeCallback)
        return callback = callback(children), invokeCallback = nameSoFar === "" ? "." + getElementKey(children, 0) : nameSoFar, isArrayImpl(callback) ? (escapedPrefix = "", invokeCallback != null && (escapedPrefix = invokeCallback.replace(userProvidedKeyEscapeRegex, "$&/") + "/"), mapIntoArray(callback, array, escapedPrefix, "", function(c) {
          return c;
        })) : callback != null && (isValidElement(callback) && (callback = cloneAndReplaceKey(callback, escapedPrefix + (callback.key == null || children && children.key === callback.key ? "" : ("" + callback.key).replace(userProvidedKeyEscapeRegex, "$&/") + "/") + invokeCallback)), array.push(callback)), 1;
      invokeCallback = 0;
      var nextNamePrefix = nameSoFar === "" ? "." : nameSoFar + ":";
      if (isArrayImpl(children))
        for (var i = 0;i < children.length; i++)
          nameSoFar = children[i], type = nextNamePrefix + getElementKey(nameSoFar, i), invokeCallback += mapIntoArray(nameSoFar, array, escapedPrefix, type, callback);
      else if (i = getIteratorFn(children), typeof i === "function")
        for (children = i.call(children), i = 0;!(nameSoFar = children.next()).done; )
          nameSoFar = nameSoFar.value, type = nextNamePrefix + getElementKey(nameSoFar, i++), invokeCallback += mapIntoArray(nameSoFar, array, escapedPrefix, type, callback);
      else if (type === "object") {
        if (typeof children.then === "function")
          return mapIntoArray(resolveThenable(children), array, escapedPrefix, nameSoFar, callback);
        array = String(children);
        throw Error("Objects are not valid as a React child (found: " + (array === "[object Object]" ? "object with keys {" + Object.keys(children).join(", ") + "}" : array) + "). If you meant to render a collection of children, use an array instead.");
      }
      return invokeCallback;
    }
    function mapChildren(children, func, context) {
      if (children == null)
        return children;
      var result = [], count = 0;
      mapIntoArray(children, result, "", "", function(child) {
        return func.call(context, child, count++);
      });
      return result;
    }
    function lazyInitializer(payload) {
      if (payload._status === -1) {
        var ctor = payload._result, thenable = ctor();
        thenable.then(function(moduleObject) {
          if (payload._status === 0 || payload._status === -1)
            payload._status = 1, payload._result = moduleObject, thenable.status === undefined && (thenable.status = "fulfilled", thenable.value = moduleObject);
        }, function(error) {
          if (payload._status === 0 || payload._status === -1)
            payload._status = 2, payload._result = error, thenable.status === undefined && (thenable.status = "rejected", thenable.reason = error);
        });
        payload._status === -1 && (payload._status = 0, payload._result = thenable);
      }
      if (payload._status === 1)
        return payload._result.default;
      throw payload._result;
    }
    var reportGlobalError = typeof reportError === "function" ? reportError : function(error) {
      if (typeof window === "object" && typeof window.ErrorEvent === "function") {
        var event = new window.ErrorEvent("error", {
          bubbles: true,
          cancelable: true,
          message: typeof error === "object" && error !== null && typeof error.message === "string" ? String(error.message) : String(error),
          error
        });
        if (!window.dispatchEvent(event))
          return;
      } else if (typeof process === "object" && typeof process.emit === "function") {
        process.emit("uncaughtException", error);
        return;
      }
      console.error(error);
    };
    function startTransition(scope) {
      var prevTransition = ReactSharedInternals.T, currentTransition = {};
      currentTransition.types = prevTransition !== null ? prevTransition.types : null;
      ReactSharedInternals.T = currentTransition;
      try {
        var returnValue = scope(), onStartTransitionFinish = ReactSharedInternals.S;
        onStartTransitionFinish !== null && onStartTransitionFinish(currentTransition, returnValue);
        typeof returnValue === "object" && returnValue !== null && typeof returnValue.then === "function" && returnValue.then(noop, reportGlobalError);
      } catch (error) {
        reportGlobalError(error);
      } finally {
        prevTransition !== null && currentTransition.types !== null && (prevTransition.types = currentTransition.types), ReactSharedInternals.T = prevTransition;
      }
    }
    function addTransitionType(type) {
      var transition = ReactSharedInternals.T;
      if (transition !== null) {
        var transitionTypes = transition.types;
        transitionTypes === null ? transition.types = [type] : transitionTypes.indexOf(type) === -1 && transitionTypes.push(type);
      } else
        startTransition(addTransitionType.bind(null, type));
    }
    var Children = {
      map: mapChildren,
      forEach: function(children, forEachFunc, forEachContext) {
        mapChildren(children, function() {
          forEachFunc.apply(this, arguments);
        }, forEachContext);
      },
      count: function(children) {
        var n = 0;
        mapChildren(children, function() {
          n++;
        });
        return n;
      },
      toArray: function(children) {
        return mapChildren(children, function(child) {
          return child;
        }) || [];
      },
      only: function(children) {
        if (!isValidElement(children))
          throw Error("React.Children.only expected to receive a single React element child.");
        return children;
      }
    };
    exports.Activity = REACT_ACTIVITY_TYPE;
    exports.Children = Children;
    exports.Component = Component;
    exports.Fragment = REACT_FRAGMENT_TYPE;
    exports.Profiler = REACT_PROFILER_TYPE;
    exports.PureComponent = PureComponent;
    exports.StrictMode = REACT_STRICT_MODE_TYPE;
    exports.Suspense = REACT_SUSPENSE_TYPE;
    exports.ViewTransition = REACT_VIEW_TRANSITION_TYPE;
    exports.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE = ReactSharedInternals;
    exports.__COMPILER_RUNTIME = {
      __proto__: null,
      c: function(size) {
        return ReactSharedInternals.H.useMemoCache(size);
      }
    };
    exports.addTransitionType = addTransitionType;
    exports.cache = function(fn) {
      return function() {
        return fn.apply(null, arguments);
      };
    };
    exports.cacheSignal = function() {
      return null;
    };
    exports.cloneElement = function(element, config, children) {
      if (element === null || element === undefined)
        throw Error("The argument must be a React element, but you passed " + element + ".");
      var props = assign({}, element.props), key = element.key;
      if (config != null)
        for (propName in config.key !== undefined && (key = "" + config.key), config)
          !hasOwnProperty.call(config, propName) || propName === "key" || propName === "__self" || propName === "__source" || propName === "ref" && config.ref === undefined || (props[propName] = config[propName]);
      var propName = arguments.length - 2;
      if (propName === 1)
        props.children = children;
      else if (1 < propName) {
        for (var childArray = Array(propName), i = 0;i < propName; i++)
          childArray[i] = arguments[i + 2];
        props.children = childArray;
      }
      return ReactElement(element.type, key, props);
    };
    exports.createContext = function(defaultValue) {
      defaultValue = {
        $$typeof: REACT_CONTEXT_TYPE,
        _currentValue: defaultValue,
        _currentValue2: defaultValue,
        _threadCount: 0,
        Provider: null,
        Consumer: null
      };
      defaultValue.Provider = defaultValue;
      defaultValue.Consumer = {
        $$typeof: REACT_CONSUMER_TYPE,
        _context: defaultValue
      };
      return defaultValue;
    };
    exports.createElement = function(type, config, children) {
      var propName, props = {}, key = null;
      if (config != null)
        for (propName in config.key !== undefined && (key = "" + config.key), config)
          hasOwnProperty.call(config, propName) && propName !== "key" && propName !== "__self" && propName !== "__source" && (props[propName] = config[propName]);
      var childrenLength = arguments.length - 2;
      if (childrenLength === 1)
        props.children = children;
      else if (1 < childrenLength) {
        for (var childArray = Array(childrenLength), i = 0;i < childrenLength; i++)
          childArray[i] = arguments[i + 2];
        props.children = childArray;
      }
      if (type && type.defaultProps)
        for (propName in childrenLength = type.defaultProps, childrenLength)
          props[propName] === undefined && (props[propName] = childrenLength[propName]);
      return ReactElement(type, key, props);
    };
    exports.createRef = function() {
      return { current: null };
    };
    exports.forwardRef = function(render) {
      return { $$typeof: REACT_FORWARD_REF_TYPE, render };
    };
    exports.isValidElement = isValidElement;
    exports.lazy = function(ctor) {
      return {
        $$typeof: REACT_LAZY_TYPE,
        _payload: { _status: -1, _result: ctor },
        _init: lazyInitializer
      };
    };
    exports.memo = function(type, compare) {
      return {
        $$typeof: REACT_MEMO_TYPE,
        type,
        compare: compare === undefined ? null : compare
      };
    };
    exports.startTransition = startTransition;
    exports.unstable_useCacheRefresh = function() {
      return ReactSharedInternals.H.useCacheRefresh();
    };
    exports.use = function(usable) {
      return ReactSharedInternals.H.use(usable);
    };
    exports.useActionState = function(action, initialState, permalink) {
      return ReactSharedInternals.H.useActionState(action, initialState, permalink);
    };
    exports.useCallback = function(callback, deps) {
      return ReactSharedInternals.H.useCallback(callback, deps);
    };
    exports.useContext = function(Context) {
      return ReactSharedInternals.H.useContext(Context);
    };
    exports.useDebugValue = function() {};
    exports.useDeferredValue = function(value, initialValue) {
      return ReactSharedInternals.H.useDeferredValue(value, initialValue);
    };
    exports.useEffect = function(create, deps) {
      return ReactSharedInternals.H.useEffect(create, deps);
    };
    exports.useEffectEvent = function(callback) {
      return ReactSharedInternals.H.useEffectEvent(callback);
    };
    exports.useId = function() {
      return ReactSharedInternals.H.useId();
    };
    exports.useImperativeHandle = function(ref, create, deps) {
      return ReactSharedInternals.H.useImperativeHandle(ref, create, deps);
    };
    exports.useInsertionEffect = function(create, deps) {
      return ReactSharedInternals.H.useInsertionEffect(create, deps);
    };
    exports.useLayoutEffect = function(create, deps) {
      return ReactSharedInternals.H.useLayoutEffect(create, deps);
    };
    exports.useMemo = function(create, deps) {
      return ReactSharedInternals.H.useMemo(create, deps);
    };
    exports.useOptimistic = function(passthrough, reducer) {
      return ReactSharedInternals.H.useOptimistic(passthrough, reducer);
    };
    exports.useReducer = function(reducer, initialArg, init) {
      return ReactSharedInternals.H.useReducer(reducer, initialArg, init);
    };
    exports.useRef = function(initialValue) {
      return ReactSharedInternals.H.useRef(initialValue);
    };
    exports.useState = function(initialState) {
      return ReactSharedInternals.H.useState(initialState);
    };
    exports.useSyncExternalStore = function(subscribe, getSnapshot, getServerSnapshot) {
      return ReactSharedInternals.H.useSyncExternalStore(subscribe, getSnapshot, getServerSnapshot);
    };
    exports.useTransition = function() {
      return ReactSharedInternals.H.useTransition();
    };
    exports.version = "19.3.1";
  });

  // ../../node_modules/.bun/react@19.3.1/node_modules/react/index.js
  var require_react = __commonJS(function(exports, module) {
    if (true) {
      module.exports = require_react_production();
    }
  });

  // ../../node_modules/.bun/react-reconciler@0.34.0+0441d6f8f57aca1e/node_modules/react-reconciler/cjs/react-reconciler-constants.production.js
  var require_react_reconciler_constants_production = __commonJS(function(exports) {
    exports.ConcurrentRoot = 1;
    exports.ContinuousEventPriority = 8;
    exports.DefaultEventPriority = 32;
    exports.DiscreteEventPriority = 2;
    exports.IdleEventPriority = 268435456;
    exports.LegacyRoot = 0;
    exports.NoEventPriority = 0;
  });

  // ../../node_modules/.bun/react-reconciler@0.34.0+0441d6f8f57aca1e/node_modules/react-reconciler/constants.js
  var require_constants = __commonJS(function(exports, module) {
    if (true) {
      module.exports = require_react_reconciler_constants_production();
    }
  });

  // ../../node_modules/.bun/scheduler@0.28.0/node_modules/scheduler/cjs/scheduler.production.js
  var require_scheduler_production = __commonJS(function(exports) {
    function push(heap, node) {
      var index = heap.length;
      heap.push(node);
      a:
        for (;0 < index; ) {
          var parentIndex = index - 1 >>> 1, parent = heap[parentIndex];
          if (0 < compare(parent, node))
            heap[parentIndex] = node, heap[index] = parent, index = parentIndex;
          else
            break a;
        }
    }
    function peek(heap) {
      return heap.length === 0 ? null : heap[0];
    }
    function pop(heap) {
      if (heap.length === 0)
        return null;
      var first = heap[0], last = heap.pop();
      if (last !== first) {
        heap[0] = last;
        a:
          for (var index = 0, length = heap.length, halfLength = length >>> 1;index < halfLength; ) {
            var leftIndex = 2 * (index + 1) - 1, left = heap[leftIndex], rightIndex = leftIndex + 1, right = heap[rightIndex];
            if (0 > compare(left, last))
              rightIndex < length && 0 > compare(right, left) ? (heap[index] = right, heap[rightIndex] = last, index = rightIndex) : (heap[index] = left, heap[leftIndex] = last, index = leftIndex);
            else if (rightIndex < length && 0 > compare(right, last))
              heap[index] = right, heap[rightIndex] = last, index = rightIndex;
            else
              break a;
          }
      }
      return first;
    }
    function compare(a, b) {
      var diff = a.sortIndex - b.sortIndex;
      return diff !== 0 ? diff : a.id - b.id;
    }
    exports.unstable_now = undefined;
    if (typeof performance === "object" && typeof performance.now === "function") {
      localPerformance = performance;
      exports.unstable_now = function() {
        return localPerformance.now();
      };
    } else {
      localDate = Date, initialTime = localDate.now();
      exports.unstable_now = function() {
        return localDate.now() - initialTime;
      };
    }
    var localPerformance;
    var localDate;
    var initialTime;
    var taskQueue = [];
    var timerQueue = [];
    var taskIdCounter = 1;
    var currentTask = null;
    var currentPriorityLevel = 3;
    var isPerformingWork = false;
    var isHostCallbackScheduled = false;
    var isHostTimeoutScheduled = false;
    var needsPaint = false;
    var localSetTimeout = typeof setTimeout === "function" ? setTimeout : null;
    var localClearTimeout = typeof clearTimeout === "function" ? clearTimeout : null;
    var localSetImmediate = typeof setImmediate !== "undefined" ? setImmediate : null;
    function advanceTimers(currentTime) {
      for (var timer = peek(timerQueue);timer !== null; ) {
        if (timer.callback === null)
          pop(timerQueue);
        else if (timer.startTime <= currentTime)
          pop(timerQueue), timer.sortIndex = timer.expirationTime, push(taskQueue, timer);
        else
          break;
        timer = peek(timerQueue);
      }
    }
    function handleTimeout(currentTime) {
      isHostTimeoutScheduled = false;
      advanceTimers(currentTime);
      if (!isHostCallbackScheduled)
        if (peek(taskQueue) !== null)
          isHostCallbackScheduled = true, isMessageLoopRunning || (isMessageLoopRunning = true, schedulePerformWorkUntilDeadline());
        else {
          var firstTimer = peek(timerQueue);
          firstTimer !== null && requestHostTimeout(handleTimeout, firstTimer.startTime - currentTime);
        }
    }
    var isMessageLoopRunning = false;
    var taskTimeoutID = -1;
    var frameInterval = 5;
    var startTime = -1;
    function shouldYieldToHost() {
      return needsPaint ? true : exports.unstable_now() - startTime < frameInterval ? false : true;
    }
    function performWorkUntilDeadline() {
      needsPaint = false;
      if (isMessageLoopRunning) {
        var currentTime = exports.unstable_now();
        startTime = currentTime;
        var hasMoreWork = true;
        try {
          a: {
            isHostCallbackScheduled = false;
            isHostTimeoutScheduled && (isHostTimeoutScheduled = false, localClearTimeout(taskTimeoutID), taskTimeoutID = -1);
            isPerformingWork = true;
            var previousPriorityLevel = currentPriorityLevel;
            try {
              b: {
                advanceTimers(currentTime);
                for (currentTask = peek(taskQueue);currentTask !== null && !(currentTask.expirationTime > currentTime && shouldYieldToHost()); ) {
                  var callback = currentTask.callback;
                  if (typeof callback === "function") {
                    currentTask.callback = null;
                    currentPriorityLevel = currentTask.priorityLevel;
                    var continuationCallback = callback(currentTask.expirationTime <= currentTime);
                    currentTime = exports.unstable_now();
                    if (typeof continuationCallback === "function") {
                      currentTask.callback = continuationCallback;
                      advanceTimers(currentTime);
                      hasMoreWork = true;
                      break b;
                    }
                    currentTask === peek(taskQueue) && pop(taskQueue);
                    advanceTimers(currentTime);
                  } else
                    pop(taskQueue);
                  currentTask = peek(taskQueue);
                }
                if (currentTask !== null)
                  hasMoreWork = true;
                else {
                  var firstTimer = peek(timerQueue);
                  firstTimer !== null && requestHostTimeout(handleTimeout, firstTimer.startTime - currentTime);
                  hasMoreWork = false;
                }
              }
              break a;
            } finally {
              currentTask = null, currentPriorityLevel = previousPriorityLevel, isPerformingWork = false;
            }
            hasMoreWork = undefined;
          }
        } finally {
          hasMoreWork ? schedulePerformWorkUntilDeadline() : isMessageLoopRunning = false;
        }
      }
    }
    var schedulePerformWorkUntilDeadline;
    if (typeof localSetImmediate === "function")
      schedulePerformWorkUntilDeadline = function() {
        localSetImmediate(performWorkUntilDeadline);
      };
    else if (typeof MessageChannel !== "undefined") {
      channel = new MessageChannel, port = channel.port2;
      channel.port1.onmessage = performWorkUntilDeadline;
      schedulePerformWorkUntilDeadline = function() {
        port.postMessage(null);
      };
    } else
      schedulePerformWorkUntilDeadline = function() {
        localSetTimeout(performWorkUntilDeadline, 0);
      };
    var channel;
    var port;
    function requestHostTimeout(callback, ms) {
      taskTimeoutID = localSetTimeout(function() {
        callback(exports.unstable_now());
      }, ms);
    }
    exports.unstable_IdlePriority = 5;
    exports.unstable_ImmediatePriority = 1;
    exports.unstable_LowPriority = 4;
    exports.unstable_NormalPriority = 3;
    exports.unstable_Profiling = null;
    exports.unstable_UserBlockingPriority = 2;
    exports.unstable_cancelCallback = function(task) {
      task.callback = null;
    };
    exports.unstable_forceFrameRate = function(fps) {
      0 > fps || 125 < fps ? console.error("forceFrameRate takes a positive int between 0 and 125, forcing frame rates higher than 125 fps is not supported") : frameInterval = 0 < fps ? Math.floor(1000 / fps) : 5;
    };
    exports.unstable_getCurrentPriorityLevel = function() {
      return currentPriorityLevel;
    };
    exports.unstable_next = function(eventHandler) {
      switch (currentPriorityLevel) {
        case 1:
        case 2:
        case 3:
          var priorityLevel = 3;
          break;
        default:
          priorityLevel = currentPriorityLevel;
      }
      var previousPriorityLevel = currentPriorityLevel;
      currentPriorityLevel = priorityLevel;
      try {
        return eventHandler();
      } finally {
        currentPriorityLevel = previousPriorityLevel;
      }
    };
    exports.unstable_requestPaint = function() {
      needsPaint = true;
    };
    exports.unstable_runWithPriority = function(priorityLevel, eventHandler) {
      switch (priorityLevel) {
        case 1:
        case 2:
        case 3:
        case 4:
        case 5:
          break;
        default:
          priorityLevel = 3;
      }
      var previousPriorityLevel = currentPriorityLevel;
      currentPriorityLevel = priorityLevel;
      try {
        return eventHandler();
      } finally {
        currentPriorityLevel = previousPriorityLevel;
      }
    };
    exports.unstable_scheduleCallback = function(priorityLevel, callback, options) {
      var currentTime = exports.unstable_now();
      typeof options === "object" && options !== null ? (options = options.delay, options = typeof options === "number" && 0 < options ? currentTime + options : currentTime) : options = currentTime;
      switch (priorityLevel) {
        case 1:
          var timeout = -1;
          break;
        case 2:
          timeout = 250;
          break;
        case 5:
          timeout = 1073741823;
          break;
        case 4:
          timeout = 1e4;
          break;
        default:
          timeout = 5000;
      }
      timeout = options + timeout;
      priorityLevel = {
        id: taskIdCounter++,
        callback,
        priorityLevel,
        startTime: options,
        expirationTime: timeout,
        sortIndex: -1
      };
      options > currentTime ? (priorityLevel.sortIndex = options, push(timerQueue, priorityLevel), peek(taskQueue) === null && priorityLevel === peek(timerQueue) && (isHostTimeoutScheduled ? (localClearTimeout(taskTimeoutID), taskTimeoutID = -1) : isHostTimeoutScheduled = true, requestHostTimeout(handleTimeout, options - currentTime))) : (priorityLevel.sortIndex = timeout, push(taskQueue, priorityLevel), isHostCallbackScheduled || isPerformingWork || (isHostCallbackScheduled = true, isMessageLoopRunning || (isMessageLoopRunning = true, schedulePerformWorkUntilDeadline())));
      return priorityLevel;
    };
    exports.unstable_shouldYield = shouldYieldToHost;
    exports.unstable_wrapCallback = function(callback) {
      var parentPriorityLevel = currentPriorityLevel;
      return function() {
        var previousPriorityLevel = currentPriorityLevel;
        currentPriorityLevel = parentPriorityLevel;
        try {
          return callback.apply(this, arguments);
        } finally {
          currentPriorityLevel = previousPriorityLevel;
        }
      };
    };
  });

  // ../../node_modules/.bun/scheduler@0.28.0/node_modules/scheduler/index.js
  var require_scheduler = __commonJS(function(exports, module) {
    if (true) {
      module.exports = require_scheduler_production();
    }
  });

  // ../../node_modules/.bun/react-reconciler@0.34.0+0441d6f8f57aca1e/node_modules/react-reconciler/cjs/react-reconciler.production.js
  var require_react_reconciler_production = __commonJS(function(exports, module) {
    module.exports = function($$$config) {
      function createFiber(tag, pendingProps, key, mode) {
        return new FiberNode(tag, pendingProps, key, mode);
      }
      function noop() {}
      function formatProdErrorMessage(code) {
        var url = "https://react.dev/errors/" + code;
        if (1 < arguments.length) {
          url += "?args[]=" + encodeURIComponent(arguments[1]);
          for (var i = 2;i < arguments.length; i++)
            url += "&args[]=" + encodeURIComponent(arguments[i]);
        }
        return "Minified React error #" + code + "; visit " + url + " for the full message or use the non-minified dev environment for full errors and additional helpful warnings.";
      }
      function getNearestMountedFiber(fiber) {
        for (var node = fiber, nextNode = node;nextNode && !nextNode.alternate; )
          node = nextNode, (node.flags & 4098) !== 0 && (fiber = node.return), nextNode = node.return;
        for (;node.return; )
          node = node.return;
        return node.tag === 3 ? fiber : null;
      }
      function assertIsMounted(fiber) {
        if (getNearestMountedFiber(fiber) !== fiber)
          throw Error(formatProdErrorMessage(188));
      }
      function findCurrentFiberUsingSlowPath(fiber) {
        var alternate = fiber.alternate;
        if (!alternate) {
          alternate = getNearestMountedFiber(fiber);
          if (alternate === null)
            throw Error(formatProdErrorMessage(188));
          return alternate !== fiber ? null : fiber;
        }
        for (var a = fiber, b = alternate;; ) {
          var parentA = a.return;
          if (parentA === null)
            break;
          var parentB = parentA.alternate;
          if (parentB === null) {
            b = parentA.return;
            if (b !== null) {
              a = b;
              continue;
            }
            break;
          }
          if (parentA.child === parentB.child) {
            for (parentB = parentA.child;parentB; ) {
              if (parentB === a)
                return assertIsMounted(parentA), fiber;
              if (parentB === b)
                return assertIsMounted(parentA), alternate;
              parentB = parentB.sibling;
            }
            throw Error(formatProdErrorMessage(188));
          }
          if (a.return !== b.return)
            a = parentA, b = parentB;
          else {
            for (var didFindChild = false, child$0 = parentA.child;child$0; ) {
              if (child$0 === a) {
                didFindChild = true;
                a = parentA;
                b = parentB;
                break;
              }
              if (child$0 === b) {
                didFindChild = true;
                b = parentA;
                a = parentB;
                break;
              }
              child$0 = child$0.sibling;
            }
            if (!didFindChild) {
              for (child$0 = parentB.child;child$0; ) {
                if (child$0 === a) {
                  didFindChild = true;
                  a = parentB;
                  b = parentA;
                  break;
                }
                if (child$0 === b) {
                  didFindChild = true;
                  b = parentB;
                  a = parentA;
                  break;
                }
                child$0 = child$0.sibling;
              }
              if (!didFindChild)
                throw Error(formatProdErrorMessage(189));
            }
          }
          if (a.alternate !== b)
            throw Error(formatProdErrorMessage(190));
        }
        if (a.tag !== 3)
          throw Error(formatProdErrorMessage(188));
        return a.stateNode.current === a ? fiber : alternate;
      }
      function findCurrentHostFiberImpl(node) {
        var tag = node.tag;
        if (tag === 5 || tag === 26 || tag === 27 || tag === 6)
          return node;
        for (node = node.child;node !== null; ) {
          tag = findCurrentHostFiberImpl(node);
          if (tag !== null)
            return tag;
          node = node.sibling;
        }
        return null;
      }
      function findCurrentHostFiberWithNoPortalsImpl(node) {
        var tag = node.tag;
        if (tag === 5 || tag === 26 || tag === 27 || tag === 6)
          return node;
        for (node = node.child;node !== null; ) {
          if (node.tag !== 4 && (tag = findCurrentHostFiberWithNoPortalsImpl(node), tag !== null))
            return tag;
          node = node.sibling;
        }
        return null;
      }
      function getIteratorFn(maybeIterable) {
        if (maybeIterable === null || typeof maybeIterable !== "object")
          return null;
        maybeIterable = MAYBE_ITERATOR_SYMBOL && maybeIterable[MAYBE_ITERATOR_SYMBOL] || maybeIterable["@@iterator"];
        return typeof maybeIterable === "function" ? maybeIterable : null;
      }
      function getComponentNameFromType(type) {
        if (type == null)
          return null;
        if (typeof type === "function")
          return type.$$typeof === REACT_CLIENT_REFERENCE ? null : type.displayName || type.name || null;
        if (typeof type === "string")
          return type;
        switch (type) {
          case REACT_FRAGMENT_TYPE:
            return "Fragment";
          case REACT_PROFILER_TYPE:
            return "Profiler";
          case REACT_STRICT_MODE_TYPE:
            return "StrictMode";
          case REACT_SUSPENSE_TYPE:
            return "Suspense";
          case REACT_SUSPENSE_LIST_TYPE:
            return "SuspenseList";
          case REACT_ACTIVITY_TYPE:
            return "Activity";
          case REACT_VIEW_TRANSITION_TYPE:
            return "ViewTransition";
        }
        if (typeof type === "object")
          switch (type.$$typeof) {
            case REACT_PORTAL_TYPE:
              return "Portal";
            case REACT_CONTEXT_TYPE:
              return type.displayName || "Context";
            case REACT_CONSUMER_TYPE:
              return (type._context.displayName || "Context") + ".Consumer";
            case REACT_FORWARD_REF_TYPE:
              var innerType = type.render;
              type = type.displayName;
              type || (type = innerType.displayName || innerType.name || "", type = type !== "" ? "ForwardRef(" + type + ")" : "ForwardRef");
              return type;
            case REACT_MEMO_TYPE:
              return innerType = type.displayName || null, innerType !== null ? innerType : getComponentNameFromType(type.type) || "Memo";
            case REACT_LAZY_TYPE:
              innerType = type._payload;
              type = type._init;
              try {
                return getComponentNameFromType(type(innerType));
              } catch (x) {}
          }
        return null;
      }
      function createCursor(defaultValue) {
        return { current: defaultValue };
      }
      function pop(cursor) {
        0 > index$jscomp$0 || (cursor.current = valueStack[index$jscomp$0], valueStack[index$jscomp$0] = null, index$jscomp$0--);
      }
      function push(cursor, value) {
        index$jscomp$0++;
        valueStack[index$jscomp$0] = cursor.current;
        cursor.current = value;
      }
      function clz32Fallback(x) {
        x >>>= 0;
        return x === 0 ? 32 : 31 - (log$1(x) / LN2 | 0) | 0;
      }
      function getHighestPriorityLanes(lanes) {
        var pendingSyncLanes = lanes & 42;
        if (pendingSyncLanes !== 0)
          return pendingSyncLanes;
        switch (lanes & -lanes) {
          case 1:
            return 1;
          case 2:
            return 2;
          case 4:
            return 4;
          case 8:
            return 8;
          case 16:
            return 16;
          case 32:
            return 32;
          case 64:
            return 64;
          case 128:
            return 128;
          case 256:
          case 512:
          case 1024:
          case 2048:
          case 4096:
          case 8192:
          case 16384:
          case 32768:
          case 65536:
          case 131072:
            return lanes & -lanes;
          case 262144:
          case 524288:
          case 1048576:
          case 2097152:
            return lanes & 3932160;
          case 4194304:
          case 8388608:
          case 16777216:
          case 33554432:
            return lanes & 62914560;
          case 67108864:
            return 67108864;
          case 134217728:
            return 134217728;
          case 268435456:
            return 268435456;
          case 536870912:
            return 536870912;
          case 1073741824:
            return 0;
          default:
            return lanes;
        }
      }
      function getNextLanes(root, wipLanes, rootHasPendingCommit) {
        var pendingLanes = root.pendingLanes;
        if (pendingLanes === 0)
          return 0;
        var nextLanes = 0, suspendedLanes = root.suspendedLanes, pingedLanes = root.pingedLanes;
        root = root.warmLanes;
        var nonIdlePendingLanes = pendingLanes & 134217727;
        nonIdlePendingLanes !== 0 ? (pendingLanes = nonIdlePendingLanes & ~suspendedLanes, pendingLanes !== 0 ? nextLanes = getHighestPriorityLanes(pendingLanes) : (pingedLanes &= nonIdlePendingLanes, pingedLanes !== 0 ? nextLanes = getHighestPriorityLanes(pingedLanes) : rootHasPendingCommit || (rootHasPendingCommit = nonIdlePendingLanes & ~root, rootHasPendingCommit !== 0 && (nextLanes = getHighestPriorityLanes(rootHasPendingCommit))))) : (nonIdlePendingLanes = pendingLanes & ~suspendedLanes, nonIdlePendingLanes !== 0 ? nextLanes = getHighestPriorityLanes(nonIdlePendingLanes) : pingedLanes !== 0 ? nextLanes = getHighestPriorityLanes(pingedLanes) : rootHasPendingCommit || (rootHasPendingCommit = pendingLanes & ~root, rootHasPendingCommit !== 0 && (nextLanes = getHighestPriorityLanes(rootHasPendingCommit))));
        return nextLanes === 0 ? 0 : wipLanes !== 0 && wipLanes !== nextLanes && (wipLanes & suspendedLanes) === 0 && (suspendedLanes = nextLanes & -nextLanes, rootHasPendingCommit = wipLanes & -wipLanes, suspendedLanes >= rootHasPendingCommit || suspendedLanes === 32 && (rootHasPendingCommit & 4194048) !== 0) ? wipLanes : nextLanes;
      }
      function checkIfRootIsPrerendering(root, renderLanes) {
        return (root.pendingLanes & ~(root.suspendedLanes & ~root.pingedLanes) & renderLanes) === 0;
      }
      function getEntangledLanes(root, renderLanes) {
        (renderLanes & 8) !== 0 && (renderLanes |= renderLanes & 32);
        var allEntangledLanes = root.entangledLanes;
        if (allEntangledLanes !== 0)
          for (root = root.entanglements, allEntangledLanes &= renderLanes;0 < allEntangledLanes; ) {
            var index$2 = 31 - clz32(allEntangledLanes), lane = 1 << index$2;
            renderLanes |= root[index$2];
            allEntangledLanes &= ~lane;
          }
        return renderLanes;
      }
      function computeExpirationTime(lane, currentTime) {
        switch (lane) {
          case 1:
          case 2:
          case 4:
          case 8:
          case 64:
            return currentTime + 250;
          case 16:
          case 32:
          case 128:
          case 256:
          case 512:
          case 1024:
          case 2048:
          case 4096:
          case 8192:
          case 16384:
          case 32768:
          case 65536:
          case 131072:
          case 262144:
          case 524288:
          case 1048576:
          case 2097152:
            return currentTime + 5000;
          case 4194304:
          case 8388608:
          case 16777216:
          case 33554432:
            return -1;
          case 67108864:
          case 134217728:
          case 268435456:
          case 536870912:
          case 1073741824:
            return -1;
          default:
            return -1;
        }
      }
      function claimNextRetryLane() {
        var lane = nextRetryLane;
        nextRetryLane <<= 1;
        (nextRetryLane & 62914560) === 0 && (nextRetryLane = 4194304);
        return lane;
      }
      function createLaneMap(initial) {
        for (var laneMap = [], i = 0;31 > i; i++)
          laneMap.push(initial);
        return laneMap;
      }
      function markRootUpdated$1(root, updateLane) {
        root.pendingLanes |= updateLane;
        updateLane !== 268435456 && (root.suspendedLanes = 0, root.pingedLanes = 0, root.warmLanes = 0);
      }
      function markRootFinished(root, finishedLanes, remainingLanes, spawnedLane, updatedLanes, suspendedRetryLanes) {
        var previouslyPendingLanes = root.pendingLanes;
        root.pendingLanes = remainingLanes;
        root.suspendedLanes = 0;
        root.pingedLanes = 0;
        root.warmLanes = 0;
        root.expiredLanes &= remainingLanes;
        root.entangledLanes &= remainingLanes;
        root.errorRecoveryDisabledLanes &= remainingLanes;
        root.shellSuspendCounter = 0;
        var { entanglements, expirationTimes, hiddenUpdates } = root;
        for (remainingLanes = previouslyPendingLanes & ~remainingLanes;0 < remainingLanes; ) {
          var index$5 = 31 - clz32(remainingLanes), lane = 1 << index$5;
          entanglements[index$5] = 0;
          expirationTimes[index$5] = -1;
          var hiddenUpdatesForLane = hiddenUpdates[index$5];
          if (hiddenUpdatesForLane !== null)
            for (hiddenUpdates[index$5] = null, index$5 = 0;index$5 < hiddenUpdatesForLane.length; index$5++) {
              var update = hiddenUpdatesForLane[index$5];
              update !== null && (update.lane &= -536870913);
            }
          remainingLanes &= ~lane;
        }
        spawnedLane !== 0 && markSpawnedDeferredLane(root, spawnedLane, 0);
        suspendedRetryLanes !== 0 && updatedLanes === 0 && root.tag !== 0 && (root.suspendedLanes |= suspendedRetryLanes & ~(previouslyPendingLanes & ~finishedLanes));
      }
      function markSpawnedDeferredLane(root, spawnedLane, entangledLanes) {
        root.pendingLanes |= spawnedLane;
        root.suspendedLanes &= ~spawnedLane;
        var spawnedLaneIndex = 31 - clz32(spawnedLane);
        root.entangledLanes |= spawnedLane;
        root.entanglements[spawnedLaneIndex] = root.entanglements[spawnedLaneIndex] | 1073741824 | entangledLanes & 261930;
      }
      function markRootEntangled(root, entangledLanes) {
        var rootEntangledLanes = root.entangledLanes |= entangledLanes;
        for (root = root.entanglements;rootEntangledLanes; ) {
          var index$6 = 31 - clz32(rootEntangledLanes), lane = 1 << index$6;
          lane & entangledLanes | root[index$6] & entangledLanes && (root[index$6] |= entangledLanes);
          rootEntangledLanes &= ~lane;
        }
      }
      function getBumpedLaneForHydration(root, renderLanes) {
        var renderLane = renderLanes & -renderLanes;
        renderLane = (renderLane & 42) !== 0 ? 1 : getBumpedLaneForHydrationByLane(renderLane);
        return (renderLane & (root.suspendedLanes | renderLanes)) !== 0 ? 0 : renderLane;
      }
      function getBumpedLaneForHydrationByLane(lane) {
        switch (lane) {
          case 2:
            lane = 1;
            break;
          case 8:
            lane = 4;
            break;
          case 32:
            lane = 16;
            break;
          case 256:
          case 512:
          case 1024:
          case 2048:
          case 4096:
          case 8192:
          case 16384:
          case 32768:
          case 65536:
          case 131072:
          case 262144:
          case 524288:
          case 1048576:
          case 2097152:
          case 4194304:
          case 8388608:
          case 16777216:
          case 33554432:
            lane = 128;
            break;
          case 268435456:
            lane = 134217728;
            break;
          default:
            lane = 0;
        }
        return lane;
      }
      function lanesToEventPriority(lanes) {
        lanes &= -lanes;
        return 2 < lanes ? 8 < lanes ? (lanes & 134217727) !== 0 ? 32 : 268435456 : 8 : 2;
      }
      function setIsStrictModeForDevtools(newIsStrictMode) {
        typeof log === "function" && unstable_setDisableYieldValue(newIsStrictMode);
        if (injectedHook && typeof injectedHook.setStrictMode === "function")
          try {
            injectedHook.setStrictMode(rendererID, newIsStrictMode);
          } catch (err) {}
      }
      function getViewTransitionName(props, instance) {
        if (props.name != null && props.name !== "auto")
          return props.name;
        if (instance.autoName !== null)
          return instance.autoName;
        props = pendingEffectsRoot.identifierPrefix;
        var globalClientId = globalClientIdCounter$1++;
        props = "_" + props + "t_" + globalClientId.toString(32) + "_";
        return instance.autoName = props;
      }
      function getClassNameByType(classByType) {
        if (classByType == null || typeof classByType === "string")
          return classByType;
        var className = null, activeTypes = pendingTransitionTypes;
        if (activeTypes !== null)
          for (var i = 0;i < activeTypes.length; i++) {
            var match = classByType[activeTypes[i]];
            if (match != null) {
              if (match === "none")
                return "none";
              className = className == null ? match : className + (" " + match);
            }
          }
        return className == null ? classByType.default : className;
      }
      function getViewTransitionClassName(defaultClass, eventClass) {
        defaultClass = getClassNameByType(defaultClass);
        eventClass = getClassNameByType(eventClass);
        return eventClass == null ? defaultClass === "auto" ? null : defaultClass : eventClass === "auto" ? null : eventClass;
      }
      function is(x, y) {
        return x === y && (x !== 0 || 1 / x === 1 / y) || x !== x && y !== y;
      }
      function describeBuiltInComponentFrame(name) {
        if (prefix === undefined)
          try {
            throw Error();
          } catch (x) {
            var match = x.stack.trim().match(/\n( *(at )?)/);
            prefix = match && match[1] || "";
            suffix = -1 < x.stack.indexOf(`
    at`) ? " (<anonymous>)" : -1 < x.stack.indexOf("@") ? "@unknown:0:0" : "";
          }
        return `
` + prefix + name + suffix;
      }
      function describeNativeComponentFrame(fn, construct) {
        if (!fn || reentry)
          return "";
        reentry = true;
        var previousPrepareStackTrace = Error.prepareStackTrace;
        Error.prepareStackTrace = undefined;
        try {
          var RunInRootFrame = {
            DetermineComponentFrameRoot: function() {
              try {
                if (construct) {
                  var Fake = function() {
                    throw Error();
                  };
                  Object.defineProperty(Fake.prototype, "props", {
                    set: function() {
                      throw Error();
                    }
                  });
                  if (typeof Reflect === "object" && Reflect.construct) {
                    try {
                      Reflect.construct(Fake, []);
                    } catch (x) {
                      var control = x;
                    }
                    Reflect.construct(fn, [], Fake);
                  } else {
                    try {
                      Fake.call();
                    } catch (x$8) {
                      control = x$8;
                    }
                    Fake = false;
                    try {
                      var prevProps = Object.getOwnPropertyDescriptor(fn.prototype, "props");
                      Object.defineProperty(fn.prototype, "props", {
                        configurable: true,
                        set: function() {
                          throw Error();
                        }
                      });
                      Fake = true;
                      new fn;
                    } finally {
                      Fake && (prevProps !== undefined ? Object.defineProperty(fn.prototype, "props", prevProps) : delete fn.prototype.props);
                    }
                  }
                } else {
                  try {
                    throw Error();
                  } catch (x$9) {
                    control = x$9;
                  }
                  (Fake = fn()) && typeof Fake.catch === "function" && Fake.catch(function() {});
                }
              } catch (sample) {
                if (sample && control && typeof sample.stack === "string")
                  return [sample.stack, control.stack];
              }
              return [null, null];
            }
          };
          RunInRootFrame.DetermineComponentFrameRoot.displayName = "DetermineComponentFrameRoot";
          var namePropDescriptor = Object.getOwnPropertyDescriptor(RunInRootFrame.DetermineComponentFrameRoot, "name");
          namePropDescriptor && namePropDescriptor.configurable && Object.defineProperty(RunInRootFrame.DetermineComponentFrameRoot, "name", { value: "DetermineComponentFrameRoot" });
          var _RunInRootFrame$Deter = RunInRootFrame.DetermineComponentFrameRoot(), sampleStack = _RunInRootFrame$Deter[0], controlStack = _RunInRootFrame$Deter[1];
          if (sampleStack && controlStack) {
            var sampleLines = sampleStack.split(`
`), controlLines = controlStack.split(`
`);
            for (namePropDescriptor = RunInRootFrame = 0;RunInRootFrame < sampleLines.length && !sampleLines[RunInRootFrame].includes("DetermineComponentFrameRoot"); )
              RunInRootFrame++;
            for (;namePropDescriptor < controlLines.length && !controlLines[namePropDescriptor].includes("DetermineComponentFrameRoot"); )
              namePropDescriptor++;
            if (RunInRootFrame === sampleLines.length || namePropDescriptor === controlLines.length)
              for (RunInRootFrame = sampleLines.length - 1, namePropDescriptor = controlLines.length - 1;1 <= RunInRootFrame && 0 <= namePropDescriptor && sampleLines[RunInRootFrame] !== controlLines[namePropDescriptor]; )
                namePropDescriptor--;
            for (;1 <= RunInRootFrame && 0 <= namePropDescriptor; RunInRootFrame--, namePropDescriptor--)
              if (sampleLines[RunInRootFrame] !== controlLines[namePropDescriptor]) {
                if (RunInRootFrame !== 1 || namePropDescriptor !== 1) {
                  do
                    if (RunInRootFrame--, namePropDescriptor--, 0 > namePropDescriptor || sampleLines[RunInRootFrame] !== controlLines[namePropDescriptor]) {
                      var frame = `
` + sampleLines[RunInRootFrame].replace(" at new ", " at ");
                      fn.displayName && frame.includes("<anonymous>") && (frame = frame.replace("<anonymous>", fn.displayName));
                      return frame;
                    }
                  while (1 <= RunInRootFrame && 0 <= namePropDescriptor);
                }
                break;
              }
          }
        } finally {
          reentry = false, Error.prepareStackTrace = previousPrepareStackTrace;
        }
        return (previousPrepareStackTrace = fn ? fn.displayName || fn.name : "") ? describeBuiltInComponentFrame(previousPrepareStackTrace) : "";
      }
      function describeFiber(fiber, childFiber) {
        switch (fiber.tag) {
          case 26:
          case 27:
          case 5:
            return describeBuiltInComponentFrame(fiber.type);
          case 16:
            return describeBuiltInComponentFrame("Lazy");
          case 13:
            return fiber.child !== childFiber && childFiber !== null ? describeBuiltInComponentFrame("Suspense Fallback") : describeBuiltInComponentFrame("Suspense");
          case 19:
            return describeBuiltInComponentFrame("SuspenseList");
          case 0:
          case 15:
            return describeNativeComponentFrame(fiber.type, false);
          case 11:
            return describeNativeComponentFrame(fiber.type.render, false);
          case 1:
            return describeNativeComponentFrame(fiber.type, true);
          case 31:
            return describeBuiltInComponentFrame("Activity");
          case 30:
            return describeBuiltInComponentFrame("ViewTransition");
          default:
            return "";
        }
      }
      function getStackByFiberInDevAndProd(workInProgress) {
        try {
          var info = "", previous = null;
          do
            info += describeFiber(workInProgress, previous), previous = workInProgress, workInProgress = workInProgress.return;
          while (workInProgress);
          return info;
        } catch (x) {
          return `
Error generating stack: ` + x.message + `
` + x.stack;
        }
      }
      function createCapturedValueAtFiber(value, source) {
        if (typeof value === "object" && value !== null) {
          var existing = CapturedStacks.get(value);
          if (existing !== undefined)
            return existing;
          source = {
            value,
            source,
            stack: getStackByFiberInDevAndProd(source)
          };
          CapturedStacks.set(value, source);
          return source;
        }
        return {
          value,
          source,
          stack: getStackByFiberInDevAndProd(source)
        };
      }
      function pushTreeFork(workInProgress, totalChildren) {
        forkStack[forkStackIndex++] = treeForkCount;
        forkStack[forkStackIndex++] = treeForkProvider;
        treeForkProvider = workInProgress;
        treeForkCount = totalChildren;
      }
      function pushTreeId(workInProgress, totalChildren, index) {
        idStack[idStackIndex++] = treeContextId;
        idStack[idStackIndex++] = treeContextOverflow;
        idStack[idStackIndex++] = treeContextProvider;
        treeContextProvider = workInProgress;
        var baseIdWithLeadingBit = treeContextId;
        workInProgress = treeContextOverflow;
        var baseLength = 32 - clz32(baseIdWithLeadingBit) - 1;
        baseIdWithLeadingBit &= ~(1 << baseLength);
        index += 1;
        var length = 32 - clz32(totalChildren) + baseLength;
        if (30 < length) {
          var numberOfOverflowBits = baseLength - baseLength % 5;
          length = (baseIdWithLeadingBit & (1 << numberOfOverflowBits) - 1).toString(32);
          baseIdWithLeadingBit >>= numberOfOverflowBits;
          baseLength -= numberOfOverflowBits;
          treeContextId = 1 << 32 - clz32(totalChildren) + baseLength | index << baseLength | baseIdWithLeadingBit;
          treeContextOverflow = length + workInProgress;
        } else
          treeContextId = 1 << length | index << baseLength | baseIdWithLeadingBit, treeContextOverflow = workInProgress;
      }
      function pushMaterializedTreeId(workInProgress) {
        workInProgress.return !== null && (pushTreeFork(workInProgress, 1), pushTreeId(workInProgress, 1, 0));
      }
      function popTreeContext(workInProgress) {
        for (;workInProgress === treeForkProvider; )
          treeForkProvider = forkStack[--forkStackIndex], forkStack[forkStackIndex] = null, treeForkCount = forkStack[--forkStackIndex], forkStack[forkStackIndex] = null;
        for (;workInProgress === treeContextProvider; )
          treeContextProvider = idStack[--idStackIndex], idStack[idStackIndex] = null, treeContextOverflow = idStack[--idStackIndex], idStack[idStackIndex] = null, treeContextId = idStack[--idStackIndex], idStack[idStackIndex] = null;
      }
      function restoreSuspendedTreeContext(workInProgress, suspendedContext) {
        idStack[idStackIndex++] = treeContextId;
        idStack[idStackIndex++] = treeContextOverflow;
        idStack[idStackIndex++] = treeContextProvider;
        treeContextId = suspendedContext.id;
        treeContextOverflow = suspendedContext.overflow;
        treeContextProvider = workInProgress;
      }
      function pushHostContainer(fiber, nextRootInstance) {
        push(rootInstanceStackCursor, nextRootInstance);
        push(contextFiberStackCursor, fiber);
        push(contextStackCursor, null);
        fiber = getRootHostContext(nextRootInstance);
        pop(contextStackCursor);
        push(contextStackCursor, fiber);
      }
      function popHostContainer() {
        pop(contextStackCursor);
        pop(contextFiberStackCursor);
        pop(rootInstanceStackCursor);
      }
      function pushHostContext(fiber) {
        var stateHook = fiber.memoizedState;
        stateHook !== null && (stateHook = stateHook.memoizedState, isPrimaryRenderer ? HostTransitionContext._currentValue = stateHook : HostTransitionContext._currentValue2 = stateHook, push(hostTransitionProviderCursor, fiber));
        stateHook = contextStackCursor.current;
        var nextContext = getChildHostContext(stateHook, fiber.type);
        stateHook !== nextContext && (push(contextFiberStackCursor, fiber), push(contextStackCursor, nextContext));
      }
      function popHostContext(fiber) {
        contextFiberStackCursor.current === fiber && (pop(contextStackCursor), pop(contextFiberStackCursor));
        hostTransitionProviderCursor.current === fiber && (pop(hostTransitionProviderCursor), isPrimaryRenderer ? HostTransitionContext._currentValue = NotPendingTransition : HostTransitionContext._currentValue2 = NotPendingTransition);
      }
      function throwOnHydrationMismatch(fiber) {
        var error = Error(formatProdErrorMessage(418, 1 < arguments.length && arguments[1] !== undefined && arguments[1] ? "text" : "HTML", ""));
        queueHydrationError(createCapturedValueAtFiber(error, fiber));
        throw HydrationMismatchException;
      }
      function prepareToHydrateHostInstance(fiber, hostContext) {
        if (!supportsHydration)
          throw Error(formatProdErrorMessage(175));
        hydrateInstance(fiber.stateNode, fiber.type, fiber.memoizedProps, hostContext, fiber) || throwOnHydrationMismatch(fiber, true);
      }
      function popToNextHostParent(fiber) {
        for (hydrationParentFiber = fiber.return;hydrationParentFiber; )
          switch (hydrationParentFiber.tag) {
            case 5:
            case 31:
            case 13:
              rootOrSingletonContext = false;
              return;
            case 27:
            case 3:
              rootOrSingletonContext = true;
              return;
            default:
              hydrationParentFiber = hydrationParentFiber.return;
          }
      }
      function popHydrationState(fiber) {
        if (!supportsHydration || fiber !== hydrationParentFiber)
          return false;
        if (!isHydrating)
          return popToNextHostParent(fiber), isHydrating = true, false;
        var tag = fiber.tag;
        supportsSingletons ? tag !== 3 && tag !== 27 && (tag !== 5 || shouldDeleteUnhydratedTailInstances(fiber.type) && !shouldSetTextContent(fiber.type, fiber.memoizedProps)) && nextHydratableInstance && throwOnHydrationMismatch(fiber) : tag !== 3 && (tag !== 5 || shouldDeleteUnhydratedTailInstances(fiber.type) && !shouldSetTextContent(fiber.type, fiber.memoizedProps)) && nextHydratableInstance && throwOnHydrationMismatch(fiber);
        popToNextHostParent(fiber);
        if (tag === 13) {
          if (!supportsHydration)
            throw Error(formatProdErrorMessage(316));
          fiber = fiber.memoizedState;
          fiber = fiber !== null ? fiber.dehydrated : null;
          if (!fiber)
            throw Error(formatProdErrorMessage(317));
          nextHydratableInstance = getNextHydratableInstanceAfterSuspenseInstance(fiber);
        } else if (tag === 31) {
          fiber = fiber.memoizedState;
          fiber = fiber !== null ? fiber.dehydrated : null;
          if (!fiber)
            throw Error(formatProdErrorMessage(317));
          nextHydratableInstance = getNextHydratableInstanceAfterActivityInstance(fiber);
        } else
          nextHydratableInstance = supportsSingletons && tag === 27 ? getNextHydratableSiblingAfterSingleton(fiber.type, nextHydratableInstance) : hydrationParentFiber ? getNextHydratableSibling(fiber.stateNode) : null;
        return true;
      }
      function resetHydrationState() {
        supportsHydration && (nextHydratableInstance = hydrationParentFiber = null, isHydrating = false);
      }
      function upgradeHydrationErrorsToRecoverable() {
        var queuedErrors = hydrationErrors;
        queuedErrors !== null && (workInProgressRootRecoverableErrors === null ? workInProgressRootRecoverableErrors = queuedErrors : workInProgressRootRecoverableErrors.push.apply(workInProgressRootRecoverableErrors, queuedErrors), hydrationErrors = null);
        return queuedErrors;
      }
      function queueHydrationError(error) {
        hydrationErrors === null ? hydrationErrors = [error] : hydrationErrors.push(error);
      }
      function pushProvider(providerFiber, context, nextValue) {
        isPrimaryRenderer ? (push(valueCursor, context._currentValue), context._currentValue = nextValue) : (push(valueCursor, context._currentValue2), context._currentValue2 = nextValue);
      }
      function popProvider(context) {
        var currentValue = valueCursor.current;
        isPrimaryRenderer ? context._currentValue = currentValue : context._currentValue2 = currentValue;
        pop(valueCursor);
      }
      function scheduleContextWorkOnParentPath(parent, renderLanes, propagationRoot) {
        for (;parent !== null; ) {
          var alternate = parent.alternate;
          (parent.childLanes & renderLanes) !== renderLanes ? (parent.childLanes |= renderLanes, alternate !== null && (alternate.childLanes |= renderLanes)) : alternate !== null && (alternate.childLanes & renderLanes) !== renderLanes && (alternate.childLanes |= renderLanes);
          if (parent === propagationRoot)
            break;
          parent = parent.return;
        }
      }
      function propagateContextChanges(workInProgress, contexts, renderLanes, forcePropagateEntireTree) {
        var fiber = workInProgress.child;
        fiber !== null && (fiber.return = workInProgress);
        for (;fiber !== null; ) {
          var list = fiber.dependencies;
          if (list !== null) {
            var nextFiber = fiber.child;
            list = list.firstContext;
            a:
              for (;list !== null; ) {
                var dependency = list;
                list = fiber;
                for (var i = 0;i < contexts.length; i++)
                  if (dependency.context === contexts[i]) {
                    list.lanes |= renderLanes;
                    dependency = list.alternate;
                    dependency !== null && (dependency.lanes |= renderLanes);
                    scheduleContextWorkOnParentPath(list.return, renderLanes, workInProgress);
                    forcePropagateEntireTree || (nextFiber = null);
                    break a;
                  }
                list = dependency.next;
              }
          } else if (fiber.tag === 18) {
            nextFiber = fiber.return;
            if (nextFiber === null)
              throw Error(formatProdErrorMessage(341));
            nextFiber.lanes |= renderLanes;
            list = nextFiber.alternate;
            list !== null && (list.lanes |= renderLanes);
            scheduleContextWorkOnParentPath(nextFiber, renderLanes, workInProgress);
            nextFiber = null;
          } else
            fiber.tag === 13 && fiber.memoizedState !== null && fiber.memoizedState.dehydrated === null ? (fiber.lanes |= renderLanes, nextFiber = fiber.alternate, nextFiber !== null && (nextFiber.lanes |= renderLanes), scheduleContextWorkOnParentPath(fiber.return, renderLanes, workInProgress), nextFiber = fiber.child, nextFiber = nextFiber !== null ? nextFiber.sibling : null) : nextFiber = fiber.child;
          if (nextFiber !== null)
            nextFiber.return = fiber;
          else
            for (nextFiber = fiber;nextFiber !== null; ) {
              if (nextFiber === workInProgress) {
                nextFiber = null;
                break;
              }
              fiber = nextFiber.sibling;
              if (fiber !== null) {
                fiber.return = nextFiber.return;
                nextFiber = fiber;
                break;
              }
              nextFiber = nextFiber.return;
            }
          fiber = nextFiber;
        }
      }
      function propagateParentContextChanges(current, workInProgress, renderLanes, forcePropagateEntireTree) {
        current = null;
        for (var parent = workInProgress, isInsidePropagationBailout = false;parent !== null; ) {
          if (!isInsidePropagationBailout) {
            if ((parent.flags & 524288) !== 0)
              isInsidePropagationBailout = true;
            else if ((parent.flags & 262144) !== 0)
              break;
          }
          if (parent.tag === 10) {
            var currentParent = parent.alternate;
            if (currentParent === null)
              throw Error(formatProdErrorMessage(387));
            currentParent = currentParent.memoizedProps;
            if (currentParent !== null) {
              var context = parent.type;
              objectIs(parent.pendingProps.value, currentParent.value) || (current !== null ? current.push(context) : current = [context]);
            }
          } else if (parent === hostTransitionProviderCursor.current) {
            currentParent = parent.alternate;
            if (currentParent === null)
              throw Error(formatProdErrorMessage(387));
            currentParent.memoizedState.memoizedState !== parent.memoizedState.memoizedState && (current !== null ? current.push(HostTransitionContext) : current = [HostTransitionContext]);
          }
          parent = parent.return;
        }
        current !== null && propagateContextChanges(workInProgress, current, renderLanes, forcePropagateEntireTree);
        workInProgress.flags |= 262144;
        return current !== null;
      }
      function checkIfContextChanged(currentDependencies) {
        for (currentDependencies = currentDependencies.firstContext;currentDependencies !== null; ) {
          var context = currentDependencies.context;
          if (!objectIs(isPrimaryRenderer ? context._currentValue : context._currentValue2, currentDependencies.memoizedValue))
            return true;
          currentDependencies = currentDependencies.next;
        }
        return false;
      }
      function prepareToReadContext(workInProgress) {
        currentlyRenderingFiber$1 = workInProgress;
        lastContextDependency = null;
        workInProgress = workInProgress.dependencies;
        workInProgress !== null && (workInProgress.firstContext = null);
      }
      function readContext(context) {
        return readContextForConsumer(currentlyRenderingFiber$1, context);
      }
      function readContextDuringReconciliation(consumer, context) {
        currentlyRenderingFiber$1 === null && prepareToReadContext(consumer);
        return readContextForConsumer(consumer, context);
      }
      function readContextForConsumer(consumer, context) {
        var value = isPrimaryRenderer ? context._currentValue : context._currentValue2;
        context = { context, memoizedValue: value, next: null };
        if (lastContextDependency === null) {
          if (consumer === null)
            throw Error(formatProdErrorMessage(308));
          lastContextDependency = context;
          consumer.dependencies = { lanes: 0, firstContext: context };
          consumer.flags |= 524288;
        } else
          lastContextDependency = lastContextDependency.next = context;
        return value;
      }
      function createCache() {
        return {
          controller: new AbortControllerLocal,
          data: new Map,
          refCount: 0
        };
      }
      function releaseCache(cache) {
        cache.refCount--;
        cache.refCount === 0 && scheduleCallback$2(NormalPriority, function() {
          cache.controller.abort();
        });
      }
      function queueTransitionTypes(root, transitionTypes) {
        if ((root.pendingLanes & 4194048) !== 0) {
          var queued = root.transitionTypes;
          queued === null && (queued = root.transitionTypes = []);
          for (root = 0;root < transitionTypes.length; root++) {
            var transitionType = transitionTypes[root];
            queued.indexOf(transitionType) === -1 && queued.push(transitionType);
          }
        }
      }
      function claimQueuedTransitionTypes(root) {
        var claimed = root.transitionTypes;
        root.transitionTypes = null;
        return claimed;
      }
      function noop$1() {}
      function ensureRootIsScheduled(root) {
        root !== lastScheduledRoot && root.next === null && (lastScheduledRoot === null ? firstScheduledRoot = lastScheduledRoot = root : lastScheduledRoot = lastScheduledRoot.next = root);
        mightHavePendingSyncWork = true;
        didScheduleMicrotask || (didScheduleMicrotask = true, scheduleImmediateRootScheduleTask());
      }
      function flushSyncWorkAcrossRoots_impl(syncTransitionLanes, onlyLegacy) {
        if (!isFlushingWork && mightHavePendingSyncWork) {
          isFlushingWork = true;
          do {
            var didPerformSomeWork = false;
            for (var root = firstScheduledRoot;root !== null; ) {
              if (!onlyLegacy)
                if (syncTransitionLanes !== 0) {
                  var pendingLanes = root.pendingLanes;
                  if (pendingLanes === 0)
                    var JSCompiler_inline_result = 0;
                  else {
                    var suspendedLanes = root.suspendedLanes, pingedLanes = root.pingedLanes;
                    JSCompiler_inline_result = (1 << 31 - clz32(42 | syncTransitionLanes) + 1) - 1;
                    JSCompiler_inline_result &= pendingLanes & ~(suspendedLanes & ~pingedLanes);
                    JSCompiler_inline_result = JSCompiler_inline_result & 201326741 ? JSCompiler_inline_result & 201326741 | 1 : JSCompiler_inline_result ? JSCompiler_inline_result | 2 : 0;
                  }
                  JSCompiler_inline_result !== 0 && (didPerformSomeWork = true, performSyncWorkOnRoot(root, JSCompiler_inline_result));
                } else
                  JSCompiler_inline_result = workInProgressRootRenderLanes, JSCompiler_inline_result = getNextLanes(root, root === workInProgressRoot ? JSCompiler_inline_result : 0, root.cancelPendingCommit !== null || root.timeoutHandle !== noTimeout), (JSCompiler_inline_result & 3) === 0 || checkIfRootIsPrerendering(root, JSCompiler_inline_result) || (didPerformSomeWork = true, performSyncWorkOnRoot(root, JSCompiler_inline_result));
              root = root.next;
            }
          } while (didPerformSomeWork);
          isFlushingWork = false;
        }
      }
      function processRootScheduleInImmediateTask() {
        processRootScheduleInMicrotask();
      }
      function processRootScheduleInMicrotask() {
        mightHavePendingSyncWork = didScheduleMicrotask = false;
        var syncTransitionLanes = 0;
        currentEventTransitionLane !== 0 && shouldAttemptEagerTransition() && (syncTransitionLanes = currentEventTransitionLane);
        for (var currentTime = now(), prev = null, root = firstScheduledRoot;root !== null; ) {
          var next = root.next, nextLanes = scheduleTaskForRootDuringMicrotask(root, currentTime);
          if (nextLanes === 0)
            root.next = null, prev === null ? firstScheduledRoot = next : prev.next = next, next === null && (lastScheduledRoot = prev);
          else if (prev = root, syncTransitionLanes !== 0 || (nextLanes & 3) !== 0)
            mightHavePendingSyncWork = true;
          root = next;
        }
        pendingEffectsStatus !== 0 && pendingEffectsStatus !== 5 || flushSyncWorkAcrossRoots_impl(syncTransitionLanes, false);
        currentEventTransitionLane !== 0 && (currentEventTransitionLane = 0);
      }
      function scheduleTaskForRootDuringMicrotask(root, currentTime) {
        for (var { suspendedLanes, pingedLanes, expirationTimes } = root, lanes = root.pendingLanes & -62914561;0 < lanes; ) {
          var index$3 = 31 - clz32(lanes), lane = 1 << index$3, expirationTime = expirationTimes[index$3];
          if (expirationTime === -1) {
            if ((lane & suspendedLanes) === 0 || (lane & pingedLanes) !== 0)
              expirationTimes[index$3] = computeExpirationTime(lane, currentTime);
          } else
            expirationTime <= currentTime && (root.expiredLanes |= lane);
          lanes &= ~lane;
        }
        currentTime = workInProgressRoot;
        suspendedLanes = workInProgressRootRenderLanes;
        suspendedLanes = getNextLanes(root, root === currentTime ? suspendedLanes : 0, root.cancelPendingCommit !== null || root.timeoutHandle !== noTimeout);
        pingedLanes = root.callbackNode;
        if (suspendedLanes === 0 || root === currentTime && (workInProgressSuspendedReason === 2 || workInProgressSuspendedReason === 9) || root.cancelPendingCommit !== null)
          return pingedLanes !== null && pingedLanes !== null && cancelCallback$1(pingedLanes), root.callbackNode = null, root.callbackPriority = 0;
        if ((suspendedLanes & 3) === 0 || checkIfRootIsPrerendering(root, suspendedLanes)) {
          currentTime = suspendedLanes & -suspendedLanes;
          if (currentTime === root.callbackPriority)
            return currentTime;
          pingedLanes !== null && cancelCallback$1(pingedLanes);
          switch (lanesToEventPriority(suspendedLanes)) {
            case 2:
            case 8:
              suspendedLanes = UserBlockingPriority;
              break;
            case 32:
              suspendedLanes = NormalPriority$1;
              break;
            case 268435456:
              suspendedLanes = IdlePriority;
              break;
            default:
              suspendedLanes = NormalPriority$1;
          }
          pingedLanes = performWorkOnRootViaSchedulerTask.bind(null, root);
          suspendedLanes = scheduleCallback$3(suspendedLanes, pingedLanes);
          root.callbackPriority = currentTime;
          root.callbackNode = suspendedLanes;
          return currentTime;
        }
        pingedLanes !== null && pingedLanes !== null && cancelCallback$1(pingedLanes);
        root.callbackPriority = 2;
        root.callbackNode = null;
        return 2;
      }
      function performWorkOnRootViaSchedulerTask(root, didTimeout) {
        if (pendingEffectsStatus !== 0 && pendingEffectsStatus !== 5)
          return root.callbackNode = null, root.callbackPriority = 0, null;
        var originalCallbackNode = root.callbackNode;
        if (flushPendingEffects() && root.callbackNode !== originalCallbackNode)
          return null;
        var workInProgressRootRenderLanes$jscomp$0 = workInProgressRootRenderLanes;
        workInProgressRootRenderLanes$jscomp$0 = getNextLanes(root, root === workInProgressRoot ? workInProgressRootRenderLanes$jscomp$0 : 0, root.cancelPendingCommit !== null || root.timeoutHandle !== noTimeout);
        if (workInProgressRootRenderLanes$jscomp$0 === 0)
          return null;
        performWorkOnRoot(root, workInProgressRootRenderLanes$jscomp$0, didTimeout);
        scheduleTaskForRootDuringMicrotask(root, now());
        return root.callbackNode != null && root.callbackNode === originalCallbackNode ? performWorkOnRootViaSchedulerTask.bind(null, root) : null;
      }
      function performSyncWorkOnRoot(root, lanes) {
        if (flushPendingEffects())
          return null;
        performWorkOnRoot(root, lanes, true);
      }
      function scheduleImmediateRootScheduleTask() {
        supportsMicrotasks ? scheduleMicrotask(function() {
          (executionContext & 6) !== 0 ? scheduleCallback$3(ImmediatePriority, processRootScheduleInImmediateTask) : processRootScheduleInMicrotask();
        }) : scheduleCallback$3(ImmediatePriority, processRootScheduleInImmediateTask);
      }
      function requestTransitionLane() {
        if (currentEventTransitionLane === 0) {
          var actionScopeLane = currentEntangledLane;
          actionScopeLane === 0 && (actionScopeLane = nextTransitionUpdateLane, nextTransitionUpdateLane <<= 1, (nextTransitionUpdateLane & 261888) === 0 && (nextTransitionUpdateLane = 256));
          currentEventTransitionLane = actionScopeLane;
        }
        return currentEventTransitionLane;
      }
      function entangleAsyncAction(transition, thenable) {
        if (currentEntangledListeners === null) {
          var entangledListeners = currentEntangledListeners = [];
          currentEntangledPendingCount = 0;
          currentEntangledLane = requestTransitionLane();
          currentEntangledActionThenable = {
            status: "pending",
            value: undefined,
            then: function(resolve) {
              entangledListeners.push(resolve);
            }
          };
        }
        currentEntangledPendingCount++;
        thenable.then(pingEngtangledActionScope, pingEngtangledActionScope);
        return thenable;
      }
      function pingEngtangledActionScope() {
        if (--currentEntangledPendingCount === 0 && (entangledTransitionTypes = null, currentEntangledListeners !== null)) {
          currentEntangledActionThenable !== null && (currentEntangledActionThenable.status = "fulfilled");
          var listeners = currentEntangledListeners;
          currentEntangledListeners = null;
          currentEntangledLane = 0;
          currentEntangledActionThenable = null;
          for (var i = 0;i < listeners.length; i++)
            (0, listeners[i])();
        }
      }
      function chainThenableValue(thenable, result) {
        var listeners = [], thenableWithOverride = {
          status: "pending",
          value: null,
          reason: null,
          then: function(resolve) {
            listeners.push(resolve);
          }
        };
        thenable.then(function() {
          thenableWithOverride.status = "fulfilled";
          thenableWithOverride.value = result;
          for (var i = 0;i < listeners.length; i++)
            (0, listeners[i])(result);
        }, function(error) {
          thenableWithOverride.status = "rejected";
          thenableWithOverride.reason = error;
          for (error = 0;error < listeners.length; error++)
            (0, listeners[error])(undefined);
        });
        return thenableWithOverride;
      }
      function peekCacheFromPool() {
        var cacheResumedFromPreviousRender = resumedCache.current;
        return cacheResumedFromPreviousRender !== null ? cacheResumedFromPreviousRender : workInProgressRoot.pooledCache;
      }
      function pushTransition(offscreenWorkInProgress, prevCachePool) {
        prevCachePool === null ? push(resumedCache, resumedCache.current) : push(resumedCache, prevCachePool.pool);
      }
      function getSuspendedCache() {
        var cacheFromPool = peekCacheFromPool();
        return cacheFromPool === null ? null : {
          parent: isPrimaryRenderer ? CacheContext._currentValue : CacheContext._currentValue2,
          pool: cacheFromPool
        };
      }
      function shallowEqual(objA, objB) {
        if (objectIs(objA, objB))
          return true;
        if (typeof objA !== "object" || objA === null || typeof objB !== "object" || objB === null)
          return false;
        var keysA = Object.keys(objA), keysB = Object.keys(objB);
        if (keysA.length !== keysB.length)
          return false;
        for (keysB = 0;keysB < keysA.length; keysB++) {
          var currentKey = keysA[keysB];
          if (!hasOwnProperty.call(objB, currentKey) || !objectIs(objA[currentKey], objB[currentKey]))
            return false;
        }
        return true;
      }
      function isThenableResolved(thenable) {
        thenable = thenable.status;
        return thenable === "fulfilled" || thenable === "rejected";
      }
      function trackUsedThenable(thenableState, thenable, index) {
        index = thenableState[index];
        index === undefined ? thenableState.push(thenable) : index !== thenable && (thenable.then(noop$1, noop$1), thenable = index);
        switch (thenable.status) {
          case "fulfilled":
            return thenable.value;
          case "rejected":
            thenableState = thenable.reason;
            checkIfUseWrappedInAsyncCatch(thenableState);
            if (thenableState === undefined && !("reason" in thenable))
              throw Error(formatProdErrorMessage(600));
            throw thenableState;
          default:
            if (typeof thenable.status === "string")
              thenable.then(noop$1, noop$1);
            else {
              thenableState = workInProgressRoot;
              if (thenableState !== null && 100 < thenableState.shellSuspendCounter)
                throw Error(formatProdErrorMessage(482));
              thenableState = thenable;
              thenableState.status = "pending";
              thenableState.then(function(fulfilledValue) {
                if (thenable.status === "pending") {
                  var fulfilledThenable = thenable;
                  fulfilledThenable.status = "fulfilled";
                  fulfilledThenable.value = fulfilledValue;
                }
              }, function(error) {
                if (thenable.status === "pending") {
                  var rejectedThenable = thenable;
                  rejectedThenable.status = "rejected";
                  rejectedThenable.reason = error;
                }
              });
            }
            switch (thenable.status) {
              case "fulfilled":
                return thenable.value;
              case "rejected":
                throw thenableState = thenable.reason, checkIfUseWrappedInAsyncCatch(thenableState), thenableState;
            }
            suspendedThenable = thenable;
            throw SuspenseException;
        }
      }
      function resolveLazy(lazyType) {
        try {
          var init = lazyType._init;
          return init(lazyType._payload);
        } catch (x) {
          if (x !== null && typeof x === "object" && typeof x.then === "function")
            throw suspendedThenable = x, SuspenseException;
          throw x;
        }
      }
      function getSuspendedThenable() {
        if (suspendedThenable === null)
          throw Error(formatProdErrorMessage(459));
        var thenable = suspendedThenable;
        suspendedThenable = null;
        return thenable;
      }
      function checkIfUseWrappedInAsyncCatch(rejectedReason) {
        if (rejectedReason === SuspenseException || rejectedReason === SuspenseActionException)
          throw Error(formatProdErrorMessage(483));
      }
      function unwrapThenable(thenable) {
        var index = thenableIndexCounter$1;
        thenableIndexCounter$1 += 1;
        thenableState$1 === null && (thenableState$1 = []);
        return trackUsedThenable(thenableState$1, thenable, index);
      }
      function coerceRef(workInProgress, element) {
        element = element.props.ref;
        workInProgress.ref = element !== undefined ? element : null;
      }
      function throwOnInvalidObjectTypeImpl(returnFiber, newChild) {
        if (newChild.$$typeof === REACT_LEGACY_ELEMENT_TYPE)
          throw Error(formatProdErrorMessage(525));
        returnFiber = Object.prototype.toString.call(newChild);
        throw Error(formatProdErrorMessage(31, returnFiber === "[object Object]" ? "object with keys {" + Object.keys(newChild).join(", ") + "}" : returnFiber));
      }
      function createChildReconciler(shouldTrackSideEffects) {
        function deleteChild(returnFiber, childToDelete) {
          if (shouldTrackSideEffects) {
            var deletions = returnFiber.deletions;
            deletions === null ? (returnFiber.deletions = [childToDelete], returnFiber.flags |= 16) : deletions.push(childToDelete);
          }
        }
        function deleteRemainingChildren(returnFiber, currentFirstChild) {
          if (!shouldTrackSideEffects)
            return null;
          for (;currentFirstChild !== null; )
            deleteChild(returnFiber, currentFirstChild), currentFirstChild = currentFirstChild.sibling;
          return null;
        }
        function mapRemainingChildren(currentFirstChild) {
          for (var existingChildren = new Map;currentFirstChild !== null; )
            currentFirstChild.key === null ? existingChildren.set(currentFirstChild.index, currentFirstChild) : existingChildren.set(currentFirstChild.key, currentFirstChild), currentFirstChild = currentFirstChild.sibling;
          return existingChildren;
        }
        function useFiber(fiber, pendingProps) {
          fiber = createWorkInProgress(fiber, pendingProps);
          fiber.index = 0;
          fiber.sibling = null;
          return fiber;
        }
        function placeChild(newFiber, lastPlacedIndex, newIndex) {
          newFiber.index = newIndex;
          if (!shouldTrackSideEffects)
            return newFiber.flags |= 1048576, lastPlacedIndex;
          newIndex = newFiber.alternate;
          if (newIndex !== null)
            return newIndex = newIndex.index, newIndex < lastPlacedIndex ? (newFiber.flags |= 2, lastPlacedIndex) : newIndex;
          newFiber.flags |= 134217730;
          return lastPlacedIndex;
        }
        function placeSingleChild(newFiber) {
          shouldTrackSideEffects && newFiber.alternate === null && (newFiber.flags |= 134217730);
          return newFiber;
        }
        function updateTextNode(returnFiber, current, textContent, lanes) {
          if (current === null || current.tag !== 6)
            return current = createFiberFromText(textContent, returnFiber.mode, lanes), current.return = returnFiber, current;
          current = useFiber(current, textContent);
          current.return = returnFiber;
          return current;
        }
        function updateElement(returnFiber, current, element, lanes) {
          var elementType = element.type;
          if (elementType === REACT_FRAGMENT_TYPE)
            return returnFiber = updateFragment(returnFiber, current, element.props.children, lanes, element.key), coerceRef(returnFiber, element), returnFiber;
          if (current !== null && (current.elementType === elementType || typeof elementType === "object" && elementType !== null && elementType.$$typeof === REACT_LAZY_TYPE && resolveLazy(elementType) === current.type))
            return current = useFiber(current, element.props), coerceRef(current, element), current.return = returnFiber, current;
          current = createFiberFromTypeAndProps(element.type, element.key, element.props, null, returnFiber.mode, lanes);
          coerceRef(current, element);
          current.return = returnFiber;
          return current;
        }
        function updatePortal(returnFiber, current, portal, lanes) {
          if (current === null || current.tag !== 4 || current.stateNode.containerInfo !== portal.containerInfo || current.stateNode.implementation !== portal.implementation)
            return current = createFiberFromPortal(portal, returnFiber.mode, lanes), current.return = returnFiber, current;
          current = useFiber(current, portal.children || []);
          current.return = returnFiber;
          return current;
        }
        function updateFragment(returnFiber, current, fragment, lanes, key) {
          if (current === null || current.tag !== 7)
            return current = createFiberFromFragment(fragment, returnFiber.mode, lanes, key), current.return = returnFiber, current;
          current = useFiber(current, fragment);
          current.return = returnFiber;
          return current;
        }
        function createChild(returnFiber, newChild, lanes) {
          if (typeof newChild === "string" && newChild !== "" || typeof newChild === "number" || typeof newChild === "bigint")
            return newChild = createFiberFromText("" + newChild, returnFiber.mode, lanes), newChild.return = returnFiber, newChild;
          if (typeof newChild === "object" && newChild !== null) {
            switch (newChild.$$typeof) {
              case REACT_ELEMENT_TYPE:
                return lanes = createFiberFromTypeAndProps(newChild.type, newChild.key, newChild.props, null, returnFiber.mode, lanes), coerceRef(lanes, newChild), lanes.return = returnFiber, lanes;
              case REACT_PORTAL_TYPE:
                return newChild = createFiberFromPortal(newChild, returnFiber.mode, lanes), newChild.return = returnFiber, newChild;
              case REACT_LAZY_TYPE:
                return newChild = resolveLazy(newChild), createChild(returnFiber, newChild, lanes);
            }
            if (isArrayImpl(newChild) || getIteratorFn(newChild))
              return newChild = createFiberFromFragment(newChild, returnFiber.mode, lanes, null), newChild.return = returnFiber, newChild;
            if (typeof newChild.then === "function")
              return createChild(returnFiber, unwrapThenable(newChild), lanes);
            if (newChild.$$typeof === REACT_CONTEXT_TYPE)
              return createChild(returnFiber, readContextDuringReconciliation(returnFiber, newChild), lanes);
            throwOnInvalidObjectTypeImpl(returnFiber, newChild);
          }
          return null;
        }
        function updateSlot(returnFiber, oldFiber, newChild, lanes) {
          var key = oldFiber !== null ? oldFiber.key : null;
          if (typeof newChild === "string" && newChild !== "" || typeof newChild === "number" || typeof newChild === "bigint")
            return key !== null ? null : updateTextNode(returnFiber, oldFiber, "" + newChild, lanes);
          if (typeof newChild === "object" && newChild !== null) {
            switch (newChild.$$typeof) {
              case REACT_ELEMENT_TYPE:
                return newChild.key === key ? updateElement(returnFiber, oldFiber, newChild, lanes) : null;
              case REACT_PORTAL_TYPE:
                return newChild.key === key ? updatePortal(returnFiber, oldFiber, newChild, lanes) : null;
              case REACT_LAZY_TYPE:
                return newChild = resolveLazy(newChild), updateSlot(returnFiber, oldFiber, newChild, lanes);
            }
            if (isArrayImpl(newChild) || getIteratorFn(newChild))
              return key !== null ? null : updateFragment(returnFiber, oldFiber, newChild, lanes, null);
            if (typeof newChild.then === "function")
              return updateSlot(returnFiber, oldFiber, unwrapThenable(newChild), lanes);
            if (newChild.$$typeof === REACT_CONTEXT_TYPE)
              return updateSlot(returnFiber, oldFiber, readContextDuringReconciliation(returnFiber, newChild), lanes);
            throwOnInvalidObjectTypeImpl(returnFiber, newChild);
          }
          return null;
        }
        function updateFromMap(existingChildren, returnFiber, newIdx, newChild, lanes) {
          if (typeof newChild === "string" && newChild !== "" || typeof newChild === "number" || typeof newChild === "bigint")
            return existingChildren = existingChildren.get(newIdx) || null, updateTextNode(returnFiber, existingChildren, "" + newChild, lanes);
          if (typeof newChild === "object" && newChild !== null) {
            switch (newChild.$$typeof) {
              case REACT_ELEMENT_TYPE:
                return existingChildren = existingChildren.get(newChild.key === null ? newIdx : newChild.key) || null, updateElement(returnFiber, existingChildren, newChild, lanes);
              case REACT_PORTAL_TYPE:
                return existingChildren = existingChildren.get(newChild.key === null ? newIdx : newChild.key) || null, updatePortal(returnFiber, existingChildren, newChild, lanes);
              case REACT_LAZY_TYPE:
                return newChild = resolveLazy(newChild), updateFromMap(existingChildren, returnFiber, newIdx, newChild, lanes);
            }
            if (isArrayImpl(newChild) || getIteratorFn(newChild))
              return existingChildren = existingChildren.get(newIdx) || null, updateFragment(returnFiber, existingChildren, newChild, lanes, null);
            if (typeof newChild.then === "function")
              return updateFromMap(existingChildren, returnFiber, newIdx, unwrapThenable(newChild), lanes);
            if (newChild.$$typeof === REACT_CONTEXT_TYPE)
              return updateFromMap(existingChildren, returnFiber, newIdx, readContextDuringReconciliation(returnFiber, newChild), lanes);
            throwOnInvalidObjectTypeImpl(returnFiber, newChild);
          }
          return null;
        }
        function reconcileChildrenArray(returnFiber, currentFirstChild, newChildren, lanes) {
          for (var resultingFirstChild = null, previousNewFiber = null, oldFiber = currentFirstChild, newIdx = currentFirstChild = 0, nextOldFiber = null;oldFiber !== null && newIdx < newChildren.length; newIdx++) {
            oldFiber.index > newIdx ? (nextOldFiber = oldFiber, oldFiber = null) : nextOldFiber = oldFiber.sibling;
            var newFiber = updateSlot(returnFiber, oldFiber, newChildren[newIdx], lanes);
            if (newFiber === null) {
              oldFiber === null && (oldFiber = nextOldFiber);
              break;
            }
            shouldTrackSideEffects && oldFiber && newFiber.alternate === null && deleteChild(returnFiber, oldFiber);
            currentFirstChild = placeChild(newFiber, currentFirstChild, newIdx);
            previousNewFiber === null ? resultingFirstChild = newFiber : previousNewFiber.sibling = newFiber;
            previousNewFiber = newFiber;
            oldFiber = nextOldFiber;
          }
          if (newIdx === newChildren.length)
            return deleteRemainingChildren(returnFiber, oldFiber), isHydrating && pushTreeFork(returnFiber, newIdx), resultingFirstChild;
          if (oldFiber === null) {
            for (;newIdx < newChildren.length; newIdx++)
              oldFiber = createChild(returnFiber, newChildren[newIdx], lanes), oldFiber !== null && (currentFirstChild = placeChild(oldFiber, currentFirstChild, newIdx), previousNewFiber === null ? resultingFirstChild = oldFiber : previousNewFiber.sibling = oldFiber, previousNewFiber = oldFiber);
            isHydrating && pushTreeFork(returnFiber, newIdx);
            return resultingFirstChild;
          }
          for (oldFiber = mapRemainingChildren(oldFiber);newIdx < newChildren.length; newIdx++)
            nextOldFiber = updateFromMap(oldFiber, returnFiber, newIdx, newChildren[newIdx], lanes), nextOldFiber !== null && (shouldTrackSideEffects && (newFiber = nextOldFiber.alternate, newFiber !== null && oldFiber.delete(newFiber.key === null ? newIdx : newFiber.key)), currentFirstChild = placeChild(nextOldFiber, currentFirstChild, newIdx), previousNewFiber === null ? resultingFirstChild = nextOldFiber : previousNewFiber.sibling = nextOldFiber, previousNewFiber = nextOldFiber);
          shouldTrackSideEffects && oldFiber.forEach(function(child) {
            return deleteChild(returnFiber, child);
          });
          isHydrating && pushTreeFork(returnFiber, newIdx);
          return resultingFirstChild;
        }
        function reconcileChildrenIterator(returnFiber, currentFirstChild, newChildren, lanes) {
          if (newChildren == null)
            throw Error(formatProdErrorMessage(151));
          for (var resultingFirstChild = null, previousNewFiber = null, oldFiber = currentFirstChild, newIdx = currentFirstChild = 0, nextOldFiber = null, step = newChildren.next();oldFiber !== null && !step.done; newIdx++, step = newChildren.next()) {
            oldFiber.index > newIdx ? (nextOldFiber = oldFiber, oldFiber = null) : nextOldFiber = oldFiber.sibling;
            var newFiber = updateSlot(returnFiber, oldFiber, step.value, lanes);
            if (newFiber === null) {
              oldFiber === null && (oldFiber = nextOldFiber);
              break;
            }
            shouldTrackSideEffects && oldFiber && newFiber.alternate === null && deleteChild(returnFiber, oldFiber);
            currentFirstChild = placeChild(newFiber, currentFirstChild, newIdx);
            previousNewFiber === null ? resultingFirstChild = newFiber : previousNewFiber.sibling = newFiber;
            previousNewFiber = newFiber;
            oldFiber = nextOldFiber;
          }
          if (step.done)
            return deleteRemainingChildren(returnFiber, oldFiber), isHydrating && pushTreeFork(returnFiber, newIdx), resultingFirstChild;
          if (oldFiber === null) {
            for (;!step.done; newIdx++, step = newChildren.next())
              step = createChild(returnFiber, step.value, lanes), step !== null && (currentFirstChild = placeChild(step, currentFirstChild, newIdx), previousNewFiber === null ? resultingFirstChild = step : previousNewFiber.sibling = step, previousNewFiber = step);
            isHydrating && pushTreeFork(returnFiber, newIdx);
            return resultingFirstChild;
          }
          for (oldFiber = mapRemainingChildren(oldFiber);!step.done; newIdx++, step = newChildren.next())
            step = updateFromMap(oldFiber, returnFiber, newIdx, step.value, lanes), step !== null && (shouldTrackSideEffects && (nextOldFiber = step.alternate, nextOldFiber !== null && oldFiber.delete(nextOldFiber.key === null ? newIdx : nextOldFiber.key)), currentFirstChild = placeChild(step, currentFirstChild, newIdx), previousNewFiber === null ? resultingFirstChild = step : previousNewFiber.sibling = step, previousNewFiber = step);
          shouldTrackSideEffects && oldFiber.forEach(function(child) {
            return deleteChild(returnFiber, child);
          });
          isHydrating && pushTreeFork(returnFiber, newIdx);
          return resultingFirstChild;
        }
        function reconcileChildFibersImpl(returnFiber, currentFirstChild, newChild, lanes) {
          typeof newChild === "object" && newChild !== null && newChild.type === REACT_FRAGMENT_TYPE && newChild.key === null && newChild.props.ref === undefined && (newChild = newChild.props.children);
          if (typeof newChild === "object" && newChild !== null) {
            switch (newChild.$$typeof) {
              case REACT_ELEMENT_TYPE:
                a: {
                  for (var key = newChild.key;currentFirstChild !== null; ) {
                    if (currentFirstChild.key === key) {
                      key = newChild.type;
                      if (key === REACT_FRAGMENT_TYPE) {
                        if (currentFirstChild.tag === 7) {
                          deleteRemainingChildren(returnFiber, currentFirstChild.sibling);
                          lanes = useFiber(currentFirstChild, newChild.props.children);
                          coerceRef(lanes, newChild);
                          lanes.return = returnFiber;
                          returnFiber = lanes;
                          break a;
                        }
                      } else if (currentFirstChild.elementType === key || typeof key === "object" && key !== null && key.$$typeof === REACT_LAZY_TYPE && resolveLazy(key) === currentFirstChild.type) {
                        deleteRemainingChildren(returnFiber, currentFirstChild.sibling);
                        lanes = useFiber(currentFirstChild, newChild.props);
                        coerceRef(lanes, newChild);
                        lanes.return = returnFiber;
                        returnFiber = lanes;
                        break a;
                      }
                      deleteRemainingChildren(returnFiber, currentFirstChild);
                      break;
                    } else
                      deleteChild(returnFiber, currentFirstChild);
                    currentFirstChild = currentFirstChild.sibling;
                  }
                  newChild.type === REACT_FRAGMENT_TYPE ? (lanes = createFiberFromFragment(newChild.props.children, returnFiber.mode, lanes, newChild.key), coerceRef(lanes, newChild), lanes.return = returnFiber, returnFiber = lanes) : (lanes = createFiberFromTypeAndProps(newChild.type, newChild.key, newChild.props, null, returnFiber.mode, lanes), coerceRef(lanes, newChild), lanes.return = returnFiber, returnFiber = lanes);
                }
                return placeSingleChild(returnFiber);
              case REACT_PORTAL_TYPE:
                a: {
                  for (key = newChild.key;currentFirstChild !== null; ) {
                    if (currentFirstChild.key === key)
                      if (currentFirstChild.tag === 4 && currentFirstChild.stateNode.containerInfo === newChild.containerInfo && currentFirstChild.stateNode.implementation === newChild.implementation) {
                        deleteRemainingChildren(returnFiber, currentFirstChild.sibling);
                        lanes = useFiber(currentFirstChild, newChild.children || []);
                        lanes.return = returnFiber;
                        returnFiber = lanes;
                        break a;
                      } else {
                        deleteRemainingChildren(returnFiber, currentFirstChild);
                        break;
                      }
                    else
                      deleteChild(returnFiber, currentFirstChild);
                    currentFirstChild = currentFirstChild.sibling;
                  }
                  lanes = createFiberFromPortal(newChild, returnFiber.mode, lanes);
                  lanes.return = returnFiber;
                  returnFiber = lanes;
                }
                return placeSingleChild(returnFiber);
              case REACT_LAZY_TYPE:
                return newChild = resolveLazy(newChild), reconcileChildFibersImpl(returnFiber, currentFirstChild, newChild, lanes);
            }
            if (isArrayImpl(newChild))
              return reconcileChildrenArray(returnFiber, currentFirstChild, newChild, lanes);
            if (getIteratorFn(newChild)) {
              key = getIteratorFn(newChild);
              if (typeof key !== "function")
                throw Error(formatProdErrorMessage(150));
              newChild = key.call(newChild);
              return reconcileChildrenIterator(returnFiber, currentFirstChild, newChild, lanes);
            }
            if (typeof newChild.then === "function")
              return reconcileChildFibersImpl(returnFiber, currentFirstChild, unwrapThenable(newChild), lanes);
            if (newChild.$$typeof === REACT_CONTEXT_TYPE)
              return reconcileChildFibersImpl(returnFiber, currentFirstChild, readContextDuringReconciliation(returnFiber, newChild), lanes);
            throwOnInvalidObjectTypeImpl(returnFiber, newChild);
          }
          return typeof newChild === "string" && newChild !== "" || typeof newChild === "number" || typeof newChild === "bigint" ? (newChild = "" + newChild, currentFirstChild !== null && currentFirstChild.tag === 6 ? (deleteRemainingChildren(returnFiber, currentFirstChild.sibling), lanes = useFiber(currentFirstChild, newChild), lanes.return = returnFiber, returnFiber = lanes) : (deleteRemainingChildren(returnFiber, currentFirstChild), lanes = createFiberFromText(newChild, returnFiber.mode, lanes), lanes.return = returnFiber, returnFiber = lanes), placeSingleChild(returnFiber)) : deleteRemainingChildren(returnFiber, currentFirstChild);
        }
        return function(returnFiber, currentFirstChild, newChild, lanes) {
          try {
            thenableIndexCounter$1 = 0;
            var firstChildFiber = reconcileChildFibersImpl(returnFiber, currentFirstChild, newChild, lanes);
            thenableState$1 = null;
            return firstChildFiber;
          } catch (x) {
            if (x === SuspenseException || x === SuspenseActionException)
              throw x;
            var fiber = createFiber(29, x, null, returnFiber.mode);
            fiber.lanes = lanes;
            fiber.return = returnFiber;
            return fiber;
          } finally {}
        };
      }
      function finishQueueingConcurrentUpdates() {
        for (var endIndex = concurrentQueuesIndex, i = concurrentlyUpdatedLanes = concurrentQueuesIndex = 0;i < endIndex; ) {
          var fiber = concurrentQueues[i];
          concurrentQueues[i++] = null;
          var queue = concurrentQueues[i];
          concurrentQueues[i++] = null;
          var update = concurrentQueues[i];
          concurrentQueues[i++] = null;
          var lane = concurrentQueues[i];
          concurrentQueues[i++] = null;
          if (queue !== null && update !== null) {
            var pending = queue.pending;
            pending === null ? update.next = update : (update.next = pending.next, pending.next = update);
            queue.pending = update;
          }
          lane !== 0 && markUpdateLaneFromFiberToRoot(fiber, update, lane);
        }
      }
      function enqueueUpdate$1(fiber, queue, update, lane) {
        concurrentQueues[concurrentQueuesIndex++] = fiber;
        concurrentQueues[concurrentQueuesIndex++] = queue;
        concurrentQueues[concurrentQueuesIndex++] = update;
        concurrentQueues[concurrentQueuesIndex++] = lane;
        concurrentlyUpdatedLanes |= lane;
        fiber.lanes |= lane;
        fiber = fiber.alternate;
        fiber !== null && (fiber.lanes |= lane);
      }
      function enqueueConcurrentHookUpdate(fiber, queue, update, lane) {
        enqueueUpdate$1(fiber, queue, update, lane);
        return getRootForUpdatedFiber(fiber);
      }
      function enqueueConcurrentRenderForLane(fiber, lane) {
        enqueueUpdate$1(fiber, null, null, lane);
        return getRootForUpdatedFiber(fiber);
      }
      function markUpdateLaneFromFiberToRoot(sourceFiber, update, lane) {
        sourceFiber.lanes |= lane;
        var alternate = sourceFiber.alternate;
        alternate !== null && (alternate.lanes |= lane);
        for (var isHidden = false, parent = sourceFiber.return;parent !== null; )
          parent.childLanes |= lane, alternate = parent.alternate, alternate !== null && (alternate.childLanes |= lane), parent.tag === 22 && (sourceFiber = parent.stateNode, sourceFiber === null || sourceFiber._visibility & 1 || (isHidden = true)), sourceFiber = parent, parent = parent.return;
        return sourceFiber.tag === 3 ? (parent = sourceFiber.stateNode, isHidden && update !== null && (isHidden = 31 - clz32(lane), sourceFiber = parent.hiddenUpdates, alternate = sourceFiber[isHidden], alternate === null ? sourceFiber[isHidden] = [update] : alternate.push(update), update.lane = lane | 536870912), parent) : null;
      }
      function getRootForUpdatedFiber(sourceFiber) {
        if (50 < nestedUpdateCount)
          throw nestedUpdateCount = 0, rootWithNestedUpdates = null, Error(formatProdErrorMessage(185));
        for (var parent = sourceFiber.return;parent !== null; )
          sourceFiber = parent, parent = sourceFiber.return;
        return sourceFiber.tag === 3 ? sourceFiber.stateNode : null;
      }
      function initializeUpdateQueue(fiber) {
        fiber.updateQueue = {
          baseState: fiber.memoizedState,
          firstBaseUpdate: null,
          lastBaseUpdate: null,
          shared: { pending: null, lanes: 0, hiddenCallbacks: null },
          callbacks: null
        };
      }
      function cloneUpdateQueue(current, workInProgress) {
        current = current.updateQueue;
        workInProgress.updateQueue === current && (workInProgress.updateQueue = {
          baseState: current.baseState,
          firstBaseUpdate: current.firstBaseUpdate,
          lastBaseUpdate: current.lastBaseUpdate,
          shared: current.shared,
          callbacks: null
        });
      }
      function createUpdate(lane) {
        return { lane, tag: 0, payload: null, callback: null, next: null };
      }
      function enqueueUpdate(fiber, update, lane) {
        var updateQueue = fiber.updateQueue;
        if (updateQueue === null)
          return null;
        updateQueue = updateQueue.shared;
        if ((executionContext & 2) !== 0) {
          var pending = updateQueue.pending;
          pending === null ? update.next = update : (update.next = pending.next, pending.next = update);
          updateQueue.pending = update;
          update = getRootForUpdatedFiber(fiber);
          markUpdateLaneFromFiberToRoot(fiber, null, lane);
          return update;
        }
        enqueueUpdate$1(fiber, updateQueue, update, lane);
        return getRootForUpdatedFiber(fiber);
      }
      function entangleTransitions(root, fiber, lane) {
        fiber = fiber.updateQueue;
        if (fiber !== null && (fiber = fiber.shared, (lane & 4194048) !== 0)) {
          var queueLanes = fiber.lanes;
          queueLanes &= root.pendingLanes;
          lane |= queueLanes;
          fiber.lanes = lane;
          markRootEntangled(root, lane);
        }
      }
      function enqueueCapturedUpdate(workInProgress, capturedUpdate) {
        var queue = workInProgress.updateQueue, current = workInProgress.alternate;
        if (current !== null && (current = current.updateQueue, queue === current)) {
          var newFirst = null, newLast = null;
          queue = queue.firstBaseUpdate;
          if (queue !== null) {
            do {
              var clone = {
                lane: queue.lane,
                tag: queue.tag,
                payload: queue.payload,
                callback: null,
                next: null
              };
              newLast === null ? newFirst = newLast = clone : newLast = newLast.next = clone;
              queue = queue.next;
            } while (queue !== null);
            newLast === null ? newFirst = newLast = capturedUpdate : newLast = newLast.next = capturedUpdate;
          } else
            newFirst = newLast = capturedUpdate;
          queue = {
            baseState: current.baseState,
            firstBaseUpdate: newFirst,
            lastBaseUpdate: newLast,
            shared: current.shared,
            callbacks: current.callbacks
          };
          workInProgress.updateQueue = queue;
          return;
        }
        workInProgress = queue.lastBaseUpdate;
        workInProgress === null ? queue.firstBaseUpdate = capturedUpdate : workInProgress.next = capturedUpdate;
        queue.lastBaseUpdate = capturedUpdate;
      }
      function suspendIfUpdateReadFromEntangledAsyncAction() {
        if (didReadFromEntangledAsyncAction) {
          var entangledActionThenable = currentEntangledActionThenable;
          if (entangledActionThenable !== null)
            throw entangledActionThenable;
        }
      }
      function processUpdateQueue(workInProgress$jscomp$0, props, instance$jscomp$0, renderLanes) {
        didReadFromEntangledAsyncAction = false;
        var queue = workInProgress$jscomp$0.updateQueue;
        hasForceUpdate = false;
        var { firstBaseUpdate, lastBaseUpdate } = queue, pendingQueue = queue.shared.pending;
        if (pendingQueue !== null) {
          queue.shared.pending = null;
          var lastPendingUpdate = pendingQueue, firstPendingUpdate = lastPendingUpdate.next;
          lastPendingUpdate.next = null;
          lastBaseUpdate === null ? firstBaseUpdate = firstPendingUpdate : lastBaseUpdate.next = firstPendingUpdate;
          lastBaseUpdate = lastPendingUpdate;
          var current = workInProgress$jscomp$0.alternate;
          current !== null && (current = current.updateQueue, pendingQueue = current.lastBaseUpdate, pendingQueue !== lastBaseUpdate && (pendingQueue === null ? current.firstBaseUpdate = firstPendingUpdate : pendingQueue.next = firstPendingUpdate, current.lastBaseUpdate = lastPendingUpdate));
        }
        if (firstBaseUpdate !== null) {
          var newState = queue.baseState;
          lastBaseUpdate = 0;
          current = firstPendingUpdate = lastPendingUpdate = null;
          pendingQueue = firstBaseUpdate;
          do {
            var updateLane = pendingQueue.lane & -536870913, isHiddenUpdate = updateLane !== pendingQueue.lane;
            if (isHiddenUpdate ? (workInProgressRootRenderLanes & updateLane) === updateLane : (renderLanes & updateLane) === updateLane) {
              updateLane !== 0 && updateLane === currentEntangledLane && (didReadFromEntangledAsyncAction = true);
              current !== null && (current = current.next = {
                lane: 0,
                tag: pendingQueue.tag,
                payload: pendingQueue.payload,
                callback: null,
                next: null
              });
              a: {
                var workInProgress = workInProgress$jscomp$0, update = pendingQueue;
                updateLane = props;
                var instance = instance$jscomp$0;
                switch (update.tag) {
                  case 1:
                    workInProgress = update.payload;
                    if (typeof workInProgress === "function") {
                      newState = workInProgress.call(instance, newState, updateLane);
                      break a;
                    }
                    newState = workInProgress;
                    break a;
                  case 3:
                    workInProgress.flags = workInProgress.flags & -65537 | 128;
                  case 0:
                    workInProgress = update.payload;
                    updateLane = typeof workInProgress === "function" ? workInProgress.call(instance, newState, updateLane) : workInProgress;
                    if (updateLane === null || updateLane === undefined)
                      break a;
                    newState = assign({}, newState, updateLane);
                    break a;
                  case 2:
                    hasForceUpdate = true;
                }
              }
              updateLane = pendingQueue.callback;
              updateLane !== null && (workInProgress$jscomp$0.flags |= 64, isHiddenUpdate && (workInProgress$jscomp$0.flags |= 8192), isHiddenUpdate = queue.callbacks, isHiddenUpdate === null ? queue.callbacks = [updateLane] : isHiddenUpdate.push(updateLane));
            } else
              isHiddenUpdate = {
                lane: updateLane,
                tag: pendingQueue.tag,
                payload: pendingQueue.payload,
                callback: pendingQueue.callback,
                next: null
              }, current === null ? (firstPendingUpdate = current = isHiddenUpdate, lastPendingUpdate = newState) : current = current.next = isHiddenUpdate, lastBaseUpdate |= updateLane;
            pendingQueue = pendingQueue.next;
            if (pendingQueue === null)
              if (pendingQueue = queue.shared.pending, pendingQueue === null)
                break;
              else
                isHiddenUpdate = pendingQueue, pendingQueue = isHiddenUpdate.next, isHiddenUpdate.next = null, queue.lastBaseUpdate = isHiddenUpdate, queue.shared.pending = null;
          } while (1);
          current === null && (lastPendingUpdate = newState);
          queue.baseState = lastPendingUpdate;
          queue.firstBaseUpdate = firstPendingUpdate;
          queue.lastBaseUpdate = current;
          firstBaseUpdate === null && (queue.shared.lanes = 0);
          workInProgressRootSkippedLanes |= lastBaseUpdate;
          workInProgress$jscomp$0.lanes = lastBaseUpdate;
          workInProgress$jscomp$0.memoizedState = newState;
        }
      }
      function callCallback(callback, context) {
        if (typeof callback !== "function")
          throw Error(formatProdErrorMessage(191, callback));
        callback.call(context);
      }
      function commitCallbacks(updateQueue, context) {
        var callbacks = updateQueue.callbacks;
        if (callbacks !== null)
          for (updateQueue.callbacks = null, updateQueue = 0;updateQueue < callbacks.length; updateQueue++)
            callCallback(callbacks[updateQueue], context);
      }
      function pushHiddenContext(fiber, context) {
        fiber = entangledRenderLanes;
        push(prevEntangledRenderLanesCursor, fiber);
        push(currentTreeHiddenStackCursor, context);
        entangledRenderLanes = fiber | context.baseLanes;
      }
      function reuseHiddenContextOnStack() {
        push(prevEntangledRenderLanesCursor, entangledRenderLanes);
        push(currentTreeHiddenStackCursor, currentTreeHiddenStackCursor.current);
      }
      function popHiddenContext() {
        entangledRenderLanes = prevEntangledRenderLanesCursor.current;
        pop(currentTreeHiddenStackCursor);
        pop(prevEntangledRenderLanesCursor);
      }
      function pushPrimaryTreeSuspenseHandler(handler) {
        var current = handler.alternate;
        push(suspenseStackCursor, suspenseStackCursor.current & 1);
        push(suspenseHandlerStackCursor, handler);
        shellBoundary === null && (current === null || currentTreeHiddenStackCursor.current !== null ? shellBoundary = handler : current.memoizedState !== null && (shellBoundary = handler));
      }
      function pushDehydratedActivitySuspenseHandler(fiber) {
        push(suspenseStackCursor, suspenseStackCursor.current);
        push(suspenseHandlerStackCursor, fiber);
        shellBoundary === null && (shellBoundary = fiber);
      }
      function pushOffscreenSuspenseHandler(fiber) {
        fiber.tag === 22 ? (push(suspenseStackCursor, suspenseStackCursor.current), push(suspenseHandlerStackCursor, fiber), shellBoundary === null && (shellBoundary = fiber)) : reuseSuspenseHandlerOnStack();
      }
      function reuseSuspenseHandlerOnStack() {
        push(suspenseStackCursor, suspenseStackCursor.current);
        push(suspenseHandlerStackCursor, suspenseHandlerStackCursor.current);
      }
      function popSuspenseHandler(fiber) {
        pop(suspenseHandlerStackCursor);
        shellBoundary === fiber && (shellBoundary = null);
        pop(suspenseStackCursor);
      }
      function pushSuspenseListContext(fiber, newContext) {
        push(suspenseHandlerStackCursor, suspenseHandlerStackCursor.current);
        push(suspenseStackCursor, newContext);
      }
      function popSuspenseListContext(fiber) {
        pop(suspenseStackCursor);
        pop(suspenseHandlerStackCursor);
        shellBoundary === fiber && (shellBoundary = null);
      }
      function findFirstSuspended(row) {
        for (var node = row;node !== null; ) {
          if (node.tag === 13) {
            var state = node.memoizedState;
            if (state !== null && (state = state.dehydrated, state === null || isSuspenseInstancePending(state) || isSuspenseInstanceFallback(state)))
              return node;
          } else if (node.tag === 19 && node.memoizedProps.revealOrder !== "independent") {
            if ((node.flags & 128) !== 0)
              return node;
          } else if (node.child !== null) {
            node.child.return = node;
            node = node.child;
            continue;
          }
          if (node === row)
            break;
          for (;node.sibling === null; ) {
            if (node.return === null || node.return === row)
              return null;
            node = node.return;
          }
          node.sibling.return = node.return;
          node = node.sibling;
        }
        return null;
      }
      function throwInvalidHookError() {
        throw Error(formatProdErrorMessage(321));
      }
      function areHookInputsEqual(nextDeps, prevDeps) {
        if (prevDeps === null)
          return false;
        for (var i = 0;i < prevDeps.length && i < nextDeps.length; i++)
          if (!objectIs(nextDeps[i], prevDeps[i]))
            return false;
        return true;
      }
      function renderWithHooks(current, workInProgress, Component, props, secondArg, nextRenderLanes) {
        renderLanes = nextRenderLanes;
        currentlyRenderingFiber = workInProgress;
        workInProgress.memoizedState = null;
        workInProgress.updateQueue = null;
        workInProgress.lanes = 0;
        ReactSharedInternals.H = current === null || current.memoizedState === null ? HooksDispatcherOnMount : HooksDispatcherOnUpdate;
        shouldDoubleInvokeUserFnsInHooksDEV = false;
        nextRenderLanes = Component(props, secondArg);
        shouldDoubleInvokeUserFnsInHooksDEV = false;
        didScheduleRenderPhaseUpdateDuringThisPass && (nextRenderLanes = renderWithHooksAgain(workInProgress, Component, props, secondArg));
        finishRenderingHooks(current);
        return nextRenderLanes;
      }
      function finishRenderingHooks(current) {
        ReactSharedInternals.H = ContextOnlyDispatcher;
        var didRenderTooFewHooks = currentHook !== null && currentHook.next !== null;
        renderLanes = 0;
        workInProgressHook = currentHook = currentlyRenderingFiber = null;
        didScheduleRenderPhaseUpdate = false;
        thenableIndexCounter = 0;
        thenableState = null;
        if (didRenderTooFewHooks)
          throw Error(formatProdErrorMessage(300));
        current === null || didReceiveUpdate || (current = current.dependencies, current !== null && checkIfContextChanged(current) && (didReceiveUpdate = true));
      }
      function renderWithHooksAgain(workInProgress, Component, props, secondArg) {
        currentlyRenderingFiber = workInProgress;
        var numberOfReRenders = 0;
        do {
          didScheduleRenderPhaseUpdateDuringThisPass && (thenableState = null);
          thenableIndexCounter = 0;
          didScheduleRenderPhaseUpdateDuringThisPass = false;
          if (25 <= numberOfReRenders)
            throw Error(formatProdErrorMessage(301));
          numberOfReRenders += 1;
          workInProgressHook = currentHook = null;
          if (workInProgress.updateQueue != null) {
            var children = workInProgress.updateQueue;
            children.lastEffect = null;
            children.events = null;
            children.stores = null;
            children.memoCache != null && (children.memoCache.index = 0);
          }
          ReactSharedInternals.H = HooksDispatcherOnRerender;
          children = Component(props, secondArg);
        } while (didScheduleRenderPhaseUpdateDuringThisPass);
        return children;
      }
      function TransitionAwareHostComponent() {
        var dispatcher = ReactSharedInternals.H, maybeThenable = dispatcher.useState()[0];
        maybeThenable = typeof maybeThenable.then === "function" ? useThenable(maybeThenable) : maybeThenable;
        dispatcher = dispatcher.useState()[0];
        (currentHook !== null ? currentHook.memoizedState : null) !== dispatcher && (currentlyRenderingFiber.flags |= 1024);
        return maybeThenable;
      }
      function checkDidRenderIdHook() {
        var didRenderIdHook = localIdCounter !== 0;
        localIdCounter = 0;
        return didRenderIdHook;
      }
      function bailoutHooks(current, workInProgress, lanes) {
        workInProgress.updateQueue = current.updateQueue;
        workInProgress.flags &= -2053;
        current.lanes &= ~lanes;
      }
      function resetHooksOnUnwind(workInProgress) {
        if (didScheduleRenderPhaseUpdate) {
          for (workInProgress = workInProgress.memoizedState;workInProgress !== null; ) {
            var queue = workInProgress.queue;
            queue !== null && (queue.pending = null);
            workInProgress = workInProgress.next;
          }
          didScheduleRenderPhaseUpdate = false;
        }
        renderLanes = 0;
        workInProgressHook = currentHook = currentlyRenderingFiber = null;
        didScheduleRenderPhaseUpdateDuringThisPass = false;
        thenableIndexCounter = localIdCounter = 0;
        thenableState = null;
      }
      function mountWorkInProgressHook() {
        var hook = {
          memoizedState: null,
          baseState: null,
          baseQueue: null,
          queue: null,
          next: null
        };
        workInProgressHook === null ? currentlyRenderingFiber.memoizedState = workInProgressHook = hook : workInProgressHook = workInProgressHook.next = hook;
        return workInProgressHook;
      }
      function updateWorkInProgressHook() {
        if (currentHook === null) {
          var nextCurrentHook = currentlyRenderingFiber.alternate;
          nextCurrentHook = nextCurrentHook !== null ? nextCurrentHook.memoizedState : null;
        } else
          nextCurrentHook = currentHook.next;
        var nextWorkInProgressHook = workInProgressHook === null ? currentlyRenderingFiber.memoizedState : workInProgressHook.next;
        if (nextWorkInProgressHook !== null)
          workInProgressHook = nextWorkInProgressHook, currentHook = nextCurrentHook;
        else {
          if (nextCurrentHook === null) {
            if (currentlyRenderingFiber.alternate === null)
              throw Error(formatProdErrorMessage(467));
            throw Error(formatProdErrorMessage(310));
          }
          currentHook = nextCurrentHook;
          nextCurrentHook = {
            memoizedState: currentHook.memoizedState,
            baseState: currentHook.baseState,
            baseQueue: currentHook.baseQueue,
            queue: currentHook.queue,
            next: null
          };
          workInProgressHook === null ? currentlyRenderingFiber.memoizedState = workInProgressHook = nextCurrentHook : workInProgressHook = workInProgressHook.next = nextCurrentHook;
        }
        return workInProgressHook;
      }
      function createFunctionComponentUpdateQueue() {
        return { lastEffect: null, events: null, stores: null, memoCache: null };
      }
      function useThenable(thenable) {
        var index = thenableIndexCounter;
        thenableIndexCounter += 1;
        thenableState === null && (thenableState = []);
        thenable = trackUsedThenable(thenableState, thenable, index);
        index = currentlyRenderingFiber;
        (workInProgressHook === null ? index.memoizedState : workInProgressHook.next) === null && (index = index.alternate, ReactSharedInternals.H = index === null || index.memoizedState === null ? HooksDispatcherOnMount : HooksDispatcherOnUpdate);
        return thenable;
      }
      function use(usable) {
        if (usable !== null && typeof usable === "object") {
          if (typeof usable.then === "function")
            return useThenable(usable);
          if (usable.$$typeof === REACT_RECOVERABLE_TYPE)
            return;
          if (usable.$$typeof === REACT_CONTEXT_TYPE)
            return readContext(usable);
        }
        throw Error(formatProdErrorMessage(438, String(usable)));
      }
      function useMemoCache(size) {
        var memoCache = null, updateQueue = currentlyRenderingFiber.updateQueue;
        updateQueue !== null && (memoCache = updateQueue.memoCache);
        if (memoCache == null) {
          var current = currentlyRenderingFiber.alternate;
          current !== null && (current = current.updateQueue, current !== null && (current = current.memoCache, current != null && (memoCache = {
            data: current.data.map(function(array) {
              return array.slice();
            }),
            index: 0
          })));
        }
        memoCache == null && (memoCache = { data: [], index: 0 });
        updateQueue === null && (updateQueue = createFunctionComponentUpdateQueue(), currentlyRenderingFiber.updateQueue = updateQueue);
        updateQueue.memoCache = memoCache;
        updateQueue = memoCache.data[memoCache.index];
        if (updateQueue === undefined)
          for (updateQueue = memoCache.data[memoCache.index] = Array(size), current = 0;current < size; current++)
            updateQueue[current] = REACT_MEMO_CACHE_SENTINEL;
        memoCache.index++;
        return updateQueue;
      }
      function basicStateReducer(state, action) {
        return typeof action === "function" ? action(state) : action;
      }
      function updateReducer(reducer) {
        var hook = updateWorkInProgressHook();
        return updateReducerImpl(hook, currentHook, reducer);
      }
      function updateReducerImpl(hook, current, reducer) {
        var queue = hook.queue;
        if (queue === null)
          throw Error(formatProdErrorMessage(311));
        queue.lastRenderedReducer = reducer;
        var baseQueue = hook.baseQueue, pendingQueue = queue.pending;
        if (pendingQueue !== null) {
          if (baseQueue !== null) {
            var baseFirst = baseQueue.next;
            baseQueue.next = pendingQueue.next;
            pendingQueue.next = baseFirst;
          }
          current.baseQueue = baseQueue = pendingQueue;
          queue.pending = null;
        }
        pendingQueue = hook.baseState;
        if (baseQueue === null)
          hook.memoizedState = pendingQueue;
        else {
          current = baseQueue.next;
          var newBaseQueueFirst = baseFirst = null, newBaseQueueLast = null, update = current, didReadFromEntangledAsyncAction$53 = false;
          do {
            var updateLane = update.lane & -536870913;
            if (updateLane !== update.lane ? (workInProgressRootRenderLanes & updateLane) === updateLane : (renderLanes & updateLane) === updateLane) {
              var revertLane = update.revertLane;
              if (revertLane === 0)
                newBaseQueueLast !== null && (newBaseQueueLast = newBaseQueueLast.next = {
                  lane: 0,
                  revertLane: 0,
                  gesture: null,
                  action: update.action,
                  hasEagerState: update.hasEagerState,
                  eagerState: update.eagerState,
                  next: null
                }), updateLane === currentEntangledLane && (didReadFromEntangledAsyncAction$53 = true);
              else if ((renderLanes & revertLane) === revertLane) {
                update = update.next;
                revertLane === currentEntangledLane && (didReadFromEntangledAsyncAction$53 = true);
                continue;
              } else
                updateLane = {
                  lane: 0,
                  revertLane: update.revertLane,
                  gesture: null,
                  action: update.action,
                  hasEagerState: update.hasEagerState,
                  eagerState: update.eagerState,
                  next: null
                }, newBaseQueueLast === null ? (newBaseQueueFirst = newBaseQueueLast = updateLane, baseFirst = pendingQueue) : newBaseQueueLast = newBaseQueueLast.next = updateLane, currentlyRenderingFiber.lanes |= revertLane, workInProgressRootSkippedLanes |= revertLane;
              updateLane = update.action;
              shouldDoubleInvokeUserFnsInHooksDEV && reducer(pendingQueue, updateLane);
              pendingQueue = update.hasEagerState ? update.eagerState : reducer(pendingQueue, updateLane);
            } else
              revertLane = {
                lane: updateLane,
                revertLane: update.revertLane,
                gesture: update.gesture,
                action: update.action,
                hasEagerState: update.hasEagerState,
                eagerState: update.eagerState,
                next: null
              }, newBaseQueueLast === null ? (newBaseQueueFirst = newBaseQueueLast = revertLane, baseFirst = pendingQueue) : newBaseQueueLast = newBaseQueueLast.next = revertLane, currentlyRenderingFiber.lanes |= updateLane, workInProgressRootSkippedLanes |= updateLane;
            update = update.next;
          } while (update !== null && update !== current);
          newBaseQueueLast === null ? baseFirst = pendingQueue : newBaseQueueLast.next = newBaseQueueFirst;
          if (!objectIs(pendingQueue, hook.memoizedState) && (didReceiveUpdate = true, didReadFromEntangledAsyncAction$53 && (reducer = currentEntangledActionThenable, reducer !== null)))
            throw reducer;
          hook.memoizedState = pendingQueue;
          hook.baseState = baseFirst;
          hook.baseQueue = newBaseQueueLast;
          queue.lastRenderedState = pendingQueue;
        }
        baseQueue === null && (queue.lanes = 0);
        return [hook.memoizedState, queue.dispatch];
      }
      function rerenderReducer(reducer) {
        var hook = updateWorkInProgressHook(), queue = hook.queue;
        if (queue === null)
          throw Error(formatProdErrorMessage(311));
        queue.lastRenderedReducer = reducer;
        var { dispatch, pending: lastRenderPhaseUpdate } = queue, newState = hook.memoizedState;
        if (lastRenderPhaseUpdate !== null) {
          queue.pending = null;
          var update = lastRenderPhaseUpdate = lastRenderPhaseUpdate.next;
          do
            newState = reducer(newState, update.action), update = update.next;
          while (update !== lastRenderPhaseUpdate);
          objectIs(newState, hook.memoizedState) || (didReceiveUpdate = true);
          hook.memoizedState = newState;
          hook.baseQueue === null && (hook.baseState = newState);
          queue.lastRenderedState = newState;
        }
        return [newState, dispatch];
      }
      function updateSyncExternalStore(subscribe, getSnapshot, getServerSnapshot) {
        var fiber = currentlyRenderingFiber, hook = updateWorkInProgressHook(), isHydrating$jscomp$0 = isHydrating;
        if (isHydrating$jscomp$0) {
          if (getServerSnapshot === undefined)
            throw Error(formatProdErrorMessage(407));
          getServerSnapshot = getServerSnapshot();
        } else
          getServerSnapshot = getSnapshot();
        var snapshotChanged = !objectIs((currentHook || hook).memoizedState, getServerSnapshot);
        snapshotChanged && (hook.memoizedState = getServerSnapshot, didReceiveUpdate = true);
        hook = hook.queue;
        updateEffect(subscribeToStore.bind(null, fiber, hook, subscribe), [
          subscribe
        ]);
        subscribe = hook.getSnapshot !== getSnapshot || snapshotChanged || workInProgressHook !== null && (workInProgressHook.memoizedState.tag & 1) !== 0;
        pushSimpleEffect(subscribe ? 9 : 8, { destroy: undefined }, updateStoreInstance.bind(null, fiber, hook, getServerSnapshot, getSnapshot), null);
        if (subscribe) {
          fiber.flags |= 2048;
          if (workInProgressRoot === null)
            throw Error(formatProdErrorMessage(349));
          isHydrating$jscomp$0 || (renderLanes & 127) !== 0 || pushStoreConsistencyCheck(fiber, getSnapshot, getServerSnapshot);
        }
        return getServerSnapshot;
      }
      function pushStoreConsistencyCheck(fiber, getSnapshot, renderedSnapshot) {
        fiber.flags |= 16384;
        fiber = { getSnapshot, value: renderedSnapshot };
        getSnapshot = currentlyRenderingFiber.updateQueue;
        getSnapshot === null ? (getSnapshot = createFunctionComponentUpdateQueue(), currentlyRenderingFiber.updateQueue = getSnapshot, getSnapshot.stores = [fiber]) : (renderedSnapshot = getSnapshot.stores, renderedSnapshot === null ? getSnapshot.stores = [fiber] : renderedSnapshot.push(fiber));
      }
      function updateStoreInstance(fiber, inst, nextSnapshot, getSnapshot) {
        inst.value = nextSnapshot;
        inst.getSnapshot = getSnapshot;
        checkIfSnapshotChanged(inst) && forceStoreRerender(fiber);
      }
      function subscribeToStore(fiber, inst, subscribe) {
        return subscribe(function() {
          checkIfSnapshotChanged(inst) && forceStoreRerender(fiber);
        });
      }
      function checkIfSnapshotChanged(inst) {
        var latestGetSnapshot = inst.getSnapshot;
        inst = inst.value;
        try {
          var nextValue = latestGetSnapshot();
          return !objectIs(inst, nextValue);
        } catch (error) {
          return true;
        }
      }
      function forceStoreRerender(fiber) {
        var root = enqueueConcurrentRenderForLane(fiber, 2);
        root !== null && scheduleUpdateOnFiber(root, fiber, 2);
      }
      function mountStateImpl(initialState) {
        var hook = mountWorkInProgressHook();
        if (typeof initialState === "function") {
          var initialStateInitializer = initialState;
          initialState = initialStateInitializer();
          if (shouldDoubleInvokeUserFnsInHooksDEV) {
            setIsStrictModeForDevtools(true);
            try {
              initialStateInitializer();
            } finally {
              setIsStrictModeForDevtools(false);
            }
          }
        }
        hook.memoizedState = hook.baseState = initialState;
        hook.queue = {
          pending: null,
          lanes: 0,
          dispatch: null,
          lastRenderedReducer: basicStateReducer,
          lastRenderedState: initialState
        };
        return hook;
      }
      function updateOptimisticImpl(hook, current, passthrough, reducer) {
        hook.baseState = passthrough;
        return updateReducerImpl(hook, currentHook, typeof reducer === "function" ? reducer : basicStateReducer);
      }
      function dispatchActionState(fiber, actionQueue, setPendingState, setState, payload) {
        if (isRenderPhaseUpdate(fiber))
          throw Error(formatProdErrorMessage(485));
        fiber = actionQueue.action;
        if (fiber !== null) {
          var actionNode = {
            payload,
            action: fiber,
            next: null,
            isTransition: true,
            status: "pending",
            value: null,
            reason: null,
            listeners: [],
            then: function(listener) {
              actionNode.listeners.push(listener);
            }
          };
          ReactSharedInternals.T !== null ? setPendingState(true) : actionNode.isTransition = false;
          setState(actionNode);
          setPendingState = actionQueue.pending;
          setPendingState === null ? (actionNode.next = actionQueue.pending = actionNode, runActionStateAction(actionQueue, actionNode)) : (actionNode.next = setPendingState.next, actionQueue.pending = setPendingState.next = actionNode);
        }
      }
      function runActionStateAction(actionQueue, node) {
        var { action, payload } = node, prevState = actionQueue.state;
        if (node.isTransition) {
          var prevTransition = ReactSharedInternals.T, currentTransition = {};
          currentTransition.types = prevTransition !== null ? prevTransition.types : null;
          ReactSharedInternals.T = currentTransition;
          try {
            var returnValue = action(prevState, payload), onStartTransitionFinish = ReactSharedInternals.S;
            onStartTransitionFinish !== null && onStartTransitionFinish(currentTransition, returnValue);
            handleActionReturnValue(actionQueue, node, returnValue);
          } catch (error) {
            onActionError(actionQueue, node, error);
          } finally {
            prevTransition !== null && currentTransition.types !== null && (prevTransition.types = currentTransition.types), ReactSharedInternals.T = prevTransition;
          }
        } else
          try {
            prevTransition = action(prevState, payload), handleActionReturnValue(actionQueue, node, prevTransition);
          } catch (error$57) {
            onActionError(actionQueue, node, error$57);
          }
      }
      function handleActionReturnValue(actionQueue, node, returnValue) {
        returnValue !== null && typeof returnValue === "object" && typeof returnValue.then === "function" ? returnValue.then(function(nextState) {
          onActionSuccess(actionQueue, node, nextState);
        }, function(error) {
          return onActionError(actionQueue, node, error);
        }) : onActionSuccess(actionQueue, node, returnValue);
      }
      function onActionSuccess(actionQueue, actionNode, nextState) {
        actionNode.status = "fulfilled";
        actionNode.value = nextState;
        notifyActionListeners(actionNode);
        actionQueue.state = nextState;
        actionNode = actionQueue.pending;
        actionNode !== null && (nextState = actionNode.next, nextState === actionNode ? actionQueue.pending = null : (nextState = nextState.next, actionNode.next = nextState, runActionStateAction(actionQueue, nextState)));
      }
      function onActionError(actionQueue, actionNode, error) {
        var last = actionQueue.pending;
        actionQueue.pending = null;
        if (last !== null) {
          last = last.next;
          do
            actionNode.status = "rejected", actionNode.reason = error, notifyActionListeners(actionNode), actionNode = actionNode.next;
          while (actionNode !== last);
        }
        actionQueue.action = null;
      }
      function notifyActionListeners(actionNode) {
        actionNode = actionNode.listeners;
        for (var i = 0;i < actionNode.length; i++)
          (0, actionNode[i])();
      }
      function actionStateReducer(oldState, newState) {
        return newState;
      }
      function mountActionState(action, initialStateProp) {
        if (isHydrating) {
          var ssrFormState = workInProgressRoot.formState;
          if (ssrFormState !== null) {
            a: {
              var JSCompiler_inline_result = currentlyRenderingFiber;
              if (isHydrating) {
                if (nextHydratableInstance) {
                  var markerInstance = canHydrateFormStateMarker(nextHydratableInstance, rootOrSingletonContext);
                  if (markerInstance) {
                    nextHydratableInstance = getNextHydratableSibling(markerInstance);
                    JSCompiler_inline_result = isFormStateMarkerMatching(markerInstance);
                    break a;
                  }
                }
                throwOnHydrationMismatch(JSCompiler_inline_result);
              }
              JSCompiler_inline_result = false;
            }
            JSCompiler_inline_result && (initialStateProp = ssrFormState[0]);
          }
        }
        ssrFormState = mountWorkInProgressHook();
        ssrFormState.memoizedState = ssrFormState.baseState = initialStateProp;
        JSCompiler_inline_result = {
          pending: null,
          lanes: 0,
          dispatch: null,
          lastRenderedReducer: actionStateReducer,
          lastRenderedState: initialStateProp
        };
        ssrFormState.queue = JSCompiler_inline_result;
        ssrFormState = dispatchSetState.bind(null, currentlyRenderingFiber, JSCompiler_inline_result);
        JSCompiler_inline_result.dispatch = ssrFormState;
        JSCompiler_inline_result = mountStateImpl(false);
        var setPendingState = dispatchOptimisticSetState.bind(null, currentlyRenderingFiber, false, JSCompiler_inline_result.queue);
        JSCompiler_inline_result = mountWorkInProgressHook();
        markerInstance = {
          state: initialStateProp,
          dispatch: null,
          action,
          pending: null
        };
        JSCompiler_inline_result.queue = markerInstance;
        ssrFormState = dispatchActionState.bind(null, currentlyRenderingFiber, markerInstance, setPendingState, ssrFormState);
        markerInstance.dispatch = ssrFormState;
        JSCompiler_inline_result.memoizedState = action;
        return [initialStateProp, ssrFormState, false];
      }
      function updateActionState(action) {
        var stateHook = updateWorkInProgressHook();
        return updateActionStateImpl(stateHook, currentHook, action);
      }
      function updateActionStateImpl(stateHook, currentStateHook, action) {
        currentStateHook = updateReducerImpl(stateHook, currentStateHook, actionStateReducer)[0];
        stateHook = updateReducer(basicStateReducer)[0];
        if (typeof currentStateHook === "object" && currentStateHook !== null && typeof currentStateHook.then === "function")
          try {
            var state = useThenable(currentStateHook);
          } catch (x) {
            if (x === SuspenseException)
              throw SuspenseActionException;
            throw x;
          }
        else
          state = currentStateHook;
        currentStateHook = updateWorkInProgressHook();
        var actionQueue = currentStateHook.queue, dispatch = actionQueue.dispatch;
        action !== currentStateHook.memoizedState && (currentlyRenderingFiber.flags |= 2048, pushSimpleEffect(9, { destroy: undefined }, actionStateActionEffect.bind(null, actionQueue, action), null));
        return [state, dispatch, stateHook];
      }
      function actionStateActionEffect(actionQueue, action) {
        actionQueue.action = action;
      }
      function rerenderActionState(action) {
        var stateHook = updateWorkInProgressHook(), currentStateHook = currentHook;
        if (currentStateHook !== null)
          return updateActionStateImpl(stateHook, currentStateHook, action);
        updateWorkInProgressHook();
        stateHook = stateHook.memoizedState;
        currentStateHook = updateWorkInProgressHook();
        var dispatch = currentStateHook.queue.dispatch;
        currentStateHook.memoizedState = action;
        return [stateHook, dispatch, false];
      }
      function pushSimpleEffect(tag, inst, create, deps) {
        tag = { tag, create, deps, inst, next: null };
        inst = currentlyRenderingFiber.updateQueue;
        inst === null && (inst = createFunctionComponentUpdateQueue(), currentlyRenderingFiber.updateQueue = inst);
        create = inst.lastEffect;
        create === null ? inst.lastEffect = tag.next = tag : (deps = create.next, create.next = tag, tag.next = deps, inst.lastEffect = tag);
        return tag;
      }
      function updateRef() {
        return updateWorkInProgressHook().memoizedState;
      }
      function mountEffectImpl(fiberFlags, hookFlags, create, deps) {
        var hook = mountWorkInProgressHook();
        currentlyRenderingFiber.flags |= fiberFlags;
        hook.memoizedState = pushSimpleEffect(1 | hookFlags, { destroy: undefined }, create, deps === undefined ? null : deps);
      }
      function updateEffectImpl(fiberFlags, hookFlags, create, deps) {
        var hook = updateWorkInProgressHook();
        deps = deps === undefined ? null : deps;
        var inst = hook.memoizedState.inst;
        currentHook !== null && deps !== null && areHookInputsEqual(deps, currentHook.memoizedState.deps) ? hook.memoizedState = pushSimpleEffect(hookFlags, inst, create, deps) : (currentlyRenderingFiber.flags |= fiberFlags, hook.memoizedState = pushSimpleEffect(1 | hookFlags, inst, create, deps));
      }
      function mountEffect(create, deps) {
        mountEffectImpl(8390656, 8, create, deps);
      }
      function updateEffect(create, deps) {
        updateEffectImpl(2048, 8, create, deps);
      }
      function useEffectEventImpl(payload) {
        currentlyRenderingFiber.flags |= 4;
        var componentUpdateQueue = currentlyRenderingFiber.updateQueue;
        if (componentUpdateQueue === null)
          componentUpdateQueue = createFunctionComponentUpdateQueue(), currentlyRenderingFiber.updateQueue = componentUpdateQueue, componentUpdateQueue.events = [payload];
        else {
          var events = componentUpdateQueue.events;
          events === null ? componentUpdateQueue.events = [payload] : events.push(payload);
        }
      }
      function updateEvent(callback) {
        var ref = updateWorkInProgressHook().memoizedState;
        useEffectEventImpl({ ref, nextImpl: callback });
        return function() {
          if ((executionContext & 2) !== 0)
            throw Error(formatProdErrorMessage(440));
          return ref.impl.apply(undefined, arguments);
        };
      }
      function updateInsertionEffect(create, deps) {
        return updateEffectImpl(4, 2, create, deps);
      }
      function updateLayoutEffect(create, deps) {
        return updateEffectImpl(4, 4, create, deps);
      }
      function imperativeHandleEffect(create, ref) {
        if (typeof ref === "function") {
          create = create();
          var refCleanup = ref(create);
          return function() {
            typeof refCleanup === "function" ? refCleanup() : ref(null);
          };
        }
        if (ref !== null && ref !== undefined)
          return create = create(), ref.current = create, function() {
            ref.current = null;
          };
      }
      function updateImperativeHandle(ref, create, deps) {
        deps = deps !== null && deps !== undefined ? deps.concat([ref]) : null;
        updateEffectImpl(4, 4, imperativeHandleEffect.bind(null, create, ref), deps);
      }
      function mountDebugValue() {}
      function updateCallback(callback, deps) {
        var hook = updateWorkInProgressHook();
        deps = deps === undefined ? null : deps;
        var prevState = hook.memoizedState;
        if (deps !== null && areHookInputsEqual(deps, prevState[1]))
          return prevState[0];
        hook.memoizedState = [callback, deps];
        return callback;
      }
      function updateMemo(nextCreate, deps) {
        var hook = updateWorkInProgressHook();
        deps = deps === undefined ? null : deps;
        var prevState = hook.memoizedState;
        if (deps !== null && areHookInputsEqual(deps, prevState[1]))
          return prevState[0];
        prevState = nextCreate();
        if (shouldDoubleInvokeUserFnsInHooksDEV) {
          setIsStrictModeForDevtools(true);
          try {
            nextCreate();
          } finally {
            setIsStrictModeForDevtools(false);
          }
        }
        hook.memoizedState = [prevState, deps];
        return prevState;
      }
      function mountDeferredValueImpl(hook, value, initialValue) {
        if (initialValue === undefined || (renderLanes & 1073741824) !== 0 && (workInProgressRootRenderLanes & 261930) === 0)
          return hook.memoizedState = value;
        hook.memoizedState = initialValue;
        hook = requestDeferredLane();
        currentlyRenderingFiber.lanes |= hook;
        workInProgressRootSkippedLanes |= hook;
        return initialValue;
      }
      function updateDeferredValueImpl(hook, prevValue, value, initialValue) {
        if (objectIs(value, prevValue))
          return value;
        if (currentTreeHiddenStackCursor.current !== null)
          return hook = mountDeferredValueImpl(hook, value, initialValue), objectIs(hook, prevValue) || (didReceiveUpdate = true), hook;
        if ((renderLanes & 106) === 0 || (renderLanes & 1073741824) !== 0 && (workInProgressRootRenderLanes & 261930) === 0)
          return didReceiveUpdate = true, hook.memoizedState = value;
        hook = requestDeferredLane();
        currentlyRenderingFiber.lanes |= hook;
        workInProgressRootSkippedLanes |= hook;
        return prevValue;
      }
      function startTransition(fiber, queue, pendingState, finishedState, callback) {
        var previousPriority = getCurrentUpdatePriority();
        setCurrentUpdatePriority(previousPriority !== 0 && 8 > previousPriority ? previousPriority : 8);
        var prevTransition = ReactSharedInternals.T, currentTransition = {};
        currentTransition.types = prevTransition !== null ? prevTransition.types : null;
        ReactSharedInternals.T = currentTransition;
        dispatchOptimisticSetState(fiber, false, queue, pendingState);
        try {
          var returnValue = callback(), onStartTransitionFinish = ReactSharedInternals.S;
          onStartTransitionFinish !== null && onStartTransitionFinish(currentTransition, returnValue);
          if (returnValue !== null && typeof returnValue === "object" && typeof returnValue.then === "function") {
            var thenableForFinishedState = chainThenableValue(returnValue, finishedState);
            dispatchSetStateInternal(fiber, queue, thenableForFinishedState, requestUpdateLane(fiber));
          } else
            dispatchSetStateInternal(fiber, queue, finishedState, requestUpdateLane(fiber));
        } catch (error) {
          dispatchSetStateInternal(fiber, queue, { then: function() {}, status: "rejected", reason: error }, requestUpdateLane());
        } finally {
          setCurrentUpdatePriority(previousPriority), prevTransition !== null && currentTransition.types !== null && (prevTransition.types = currentTransition.types), ReactSharedInternals.T = prevTransition;
        }
      }
      function ensureFormComponentIsStateful(formFiber) {
        var existingStateHook = formFiber.memoizedState;
        if (existingStateHook !== null)
          return existingStateHook;
        existingStateHook = {
          memoizedState: NotPendingTransition,
          baseState: NotPendingTransition,
          baseQueue: null,
          queue: {
            pending: null,
            lanes: 0,
            dispatch: null,
            lastRenderedReducer: basicStateReducer,
            lastRenderedState: NotPendingTransition
          },
          next: null
        };
        var initialResetState = {};
        existingStateHook.next = {
          memoizedState: initialResetState,
          baseState: initialResetState,
          baseQueue: null,
          queue: {
            pending: null,
            lanes: 0,
            dispatch: null,
            lastRenderedReducer: basicStateReducer,
            lastRenderedState: initialResetState
          },
          next: null
        };
        formFiber.memoizedState = existingStateHook;
        formFiber = formFiber.alternate;
        formFiber !== null && (formFiber.memoizedState = existingStateHook);
        return existingStateHook;
      }
      function useHostTransitionStatus() {
        return readContext(HostTransitionContext);
      }
      function updateId() {
        return updateWorkInProgressHook().memoizedState;
      }
      function updateRefresh() {
        return updateWorkInProgressHook().memoizedState;
      }
      function refreshCache(fiber) {
        for (var provider = fiber.return;provider !== null; ) {
          switch (provider.tag) {
            case 24:
            case 3:
              var lane = requestUpdateLane();
              fiber = createUpdate(lane);
              var root = enqueueUpdate(provider, fiber, lane);
              root !== null && (scheduleUpdateOnFiber(root, provider, lane), entangleTransitions(root, provider, lane));
              provider = { cache: createCache() };
              fiber.payload = provider;
              return;
          }
          provider = provider.return;
        }
      }
      function dispatchReducerAction(fiber, queue, action) {
        var lane = requestUpdateLane();
        action = {
          lane,
          revertLane: 0,
          gesture: null,
          action,
          hasEagerState: false,
          eagerState: null,
          next: null
        };
        isRenderPhaseUpdate(fiber) ? enqueueRenderPhaseUpdate(queue, action) : (action = enqueueConcurrentHookUpdate(fiber, queue, action, lane), action !== null && (scheduleUpdateOnFiber(action, fiber, lane), entangleTransitionUpdate(action, queue, lane)));
      }
      function dispatchSetState(fiber, queue, action) {
        var lane = requestUpdateLane();
        dispatchSetStateInternal(fiber, queue, action, lane);
      }
      function dispatchSetStateInternal(fiber, queue, action, lane) {
        var update = {
          lane,
          revertLane: 0,
          gesture: null,
          action,
          hasEagerState: false,
          eagerState: null,
          next: null
        };
        if (isRenderPhaseUpdate(fiber))
          enqueueRenderPhaseUpdate(queue, update);
        else {
          var alternate = fiber.alternate;
          if (fiber.lanes === 0 && (alternate === null || alternate.lanes === 0) && (alternate = queue.lastRenderedReducer, alternate !== null))
            try {
              var currentState = queue.lastRenderedState, eagerState = alternate(currentState, action);
              update.hasEagerState = true;
              update.eagerState = eagerState;
              if (objectIs(eagerState, currentState))
                return enqueueUpdate$1(fiber, queue, update, 0), workInProgressRoot === null && finishQueueingConcurrentUpdates(), false;
            } catch (error) {} finally {}
          action = enqueueConcurrentHookUpdate(fiber, queue, update, lane);
          if (action !== null)
            return scheduleUpdateOnFiber(action, fiber, lane), entangleTransitionUpdate(action, queue, lane), true;
        }
        return false;
      }
      function dispatchOptimisticSetState(fiber, throwIfDuringRender, queue, action) {
        action = {
          lane: 2,
          revertLane: requestTransitionLane(),
          gesture: null,
          action,
          hasEagerState: false,
          eagerState: null,
          next: null
        };
        if (isRenderPhaseUpdate(fiber)) {
          if (throwIfDuringRender)
            throw Error(formatProdErrorMessage(479));
        } else
          throwIfDuringRender = enqueueConcurrentHookUpdate(fiber, queue, action, 2), throwIfDuringRender !== null && scheduleUpdateOnFiber(throwIfDuringRender, fiber, 2);
      }
      function isRenderPhaseUpdate(fiber) {
        var alternate = fiber.alternate;
        return fiber === currentlyRenderingFiber || alternate !== null && alternate === currentlyRenderingFiber;
      }
      function enqueueRenderPhaseUpdate(queue, update) {
        didScheduleRenderPhaseUpdateDuringThisPass = didScheduleRenderPhaseUpdate = true;
        var pending = queue.pending;
        pending === null ? update.next = update : (update.next = pending.next, pending.next = update);
        queue.pending = update;
      }
      function entangleTransitionUpdate(root, queue, lane) {
        if ((lane & 4194048) !== 0) {
          var queueLanes = queue.lanes;
          queueLanes &= root.pendingLanes;
          lane |= queueLanes;
          queue.lanes = lane;
          markRootEntangled(root, lane);
        }
      }
      function applyDerivedStateFromProps(workInProgress, ctor, getDerivedStateFromProps, nextProps) {
        ctor = workInProgress.memoizedState;
        getDerivedStateFromProps = getDerivedStateFromProps(nextProps, ctor);
        getDerivedStateFromProps = getDerivedStateFromProps === null || getDerivedStateFromProps === undefined ? ctor : assign({}, ctor, getDerivedStateFromProps);
        workInProgress.memoizedState = getDerivedStateFromProps;
        workInProgress.lanes === 0 && (workInProgress.updateQueue.baseState = getDerivedStateFromProps);
      }
      function checkShouldComponentUpdate(workInProgress, ctor, oldProps, newProps, oldState, newState, nextContext) {
        workInProgress = workInProgress.stateNode;
        return typeof workInProgress.shouldComponentUpdate === "function" ? workInProgress.shouldComponentUpdate(newProps, newState, nextContext) : ctor.prototype && ctor.prototype.isPureReactComponent ? !shallowEqual(oldProps, newProps) || !shallowEqual(oldState, newState) : true;
      }
      function callComponentWillReceiveProps(workInProgress, instance, newProps, nextContext) {
        workInProgress = instance.state;
        typeof instance.componentWillReceiveProps === "function" && instance.componentWillReceiveProps(newProps, nextContext);
        typeof instance.UNSAFE_componentWillReceiveProps === "function" && instance.UNSAFE_componentWillReceiveProps(newProps, nextContext);
        instance.state !== workInProgress && classComponentUpdater.enqueueReplaceState(instance, instance.state, null);
      }
      function resolveClassComponentProps(Component, baseProps) {
        var newProps = baseProps;
        if ("ref" in baseProps) {
          newProps = {};
          for (var propName in baseProps)
            propName !== "ref" && (newProps[propName] = baseProps[propName]);
        }
        if (Component = Component.defaultProps) {
          newProps === baseProps && (newProps = assign({}, newProps));
          for (var propName$59 in Component)
            newProps[propName$59] === undefined && (newProps[propName$59] = Component[propName$59]);
        }
        return newProps;
      }
      function logUncaughtError(root, errorInfo) {
        try {
          var onUncaughtError = root.onUncaughtError;
          onUncaughtError(errorInfo.value, { componentStack: errorInfo.stack });
        } catch (e) {
          setTimeout(function() {
            throw e;
          });
        }
      }
      function logCaughtError(root, boundary, errorInfo) {
        try {
          var onCaughtError = root.onCaughtError;
          onCaughtError(errorInfo.value, {
            componentStack: errorInfo.stack,
            errorBoundary: boundary.tag === 1 ? boundary.stateNode : null
          });
        } catch (e) {
          setTimeout(function() {
            throw e;
          });
        }
      }
      function createRootErrorUpdate(root, errorInfo, lane) {
        lane = createUpdate(lane);
        lane.tag = 3;
        lane.payload = { element: null };
        lane.callback = function() {
          logUncaughtError(root, errorInfo);
        };
        return lane;
      }
      function createClassErrorUpdate(lane) {
        lane = createUpdate(lane);
        lane.tag = 3;
        return lane;
      }
      function initializeClassErrorUpdate(update, root, fiber, errorInfo) {
        var getDerivedStateFromError = fiber.type.getDerivedStateFromError;
        if (typeof getDerivedStateFromError === "function") {
          var error = errorInfo.value;
          update.payload = function() {
            return getDerivedStateFromError(error);
          };
          update.callback = function() {
            logCaughtError(root, fiber, errorInfo);
          };
        }
        var inst = fiber.stateNode;
        inst !== null && typeof inst.componentDidCatch === "function" && (update.callback = function() {
          logCaughtError(root, fiber, errorInfo);
          typeof getDerivedStateFromError !== "function" && (legacyErrorBoundariesThatAlreadyFailed === null ? legacyErrorBoundariesThatAlreadyFailed = new Set([this]) : legacyErrorBoundariesThatAlreadyFailed.add(this));
          var stack = errorInfo.stack;
          this.componentDidCatch(errorInfo.value, {
            componentStack: stack !== null ? stack : ""
          });
        });
      }
      function throwException(root, returnFiber, sourceFiber, value, rootRenderLanes) {
        sourceFiber.flags |= 32768;
        if (value !== null && typeof value === "object" && typeof value.then === "function") {
          returnFiber = sourceFiber.alternate;
          returnFiber !== null && propagateParentContextChanges(returnFiber, sourceFiber, rootRenderLanes, true);
          sourceFiber = suspenseHandlerStackCursor.current;
          if (sourceFiber !== null) {
            switch (sourceFiber.tag) {
              case 31:
              case 13:
              case 19:
                return shellBoundary === null ? renderDidSuspendDelayIfPossible() : sourceFiber.alternate === null && workInProgressRootExitStatus === 0 && (workInProgressRootExitStatus = 3), sourceFiber.flags &= -257, sourceFiber.flags |= 65536, sourceFiber.lanes = rootRenderLanes, value === noopSuspenseyCommitThenable ? sourceFiber.flags |= 16384 : (returnFiber = sourceFiber.updateQueue, returnFiber === null ? sourceFiber.updateQueue = new Set([value]) : returnFiber.add(value), attachPingListener(root, value, rootRenderLanes)), false;
              case 22:
                return sourceFiber.flags |= 65536, value === noopSuspenseyCommitThenable ? sourceFiber.flags |= 16384 : (returnFiber = sourceFiber.updateQueue, returnFiber === null ? (returnFiber = {
                  transitions: null,
                  markerInstances: null,
                  retryQueue: new Set([value])
                }, sourceFiber.updateQueue = returnFiber) : (sourceFiber = returnFiber.retryQueue, sourceFiber === null ? returnFiber.retryQueue = new Set([value]) : sourceFiber.add(value)), attachPingListener(root, value, rootRenderLanes)), false;
            }
            throw Error(formatProdErrorMessage(435, sourceFiber.tag));
          }
          attachPingListener(root, value, rootRenderLanes);
          renderDidSuspendDelayIfPossible();
          return false;
        }
        if (isHydrating)
          return returnFiber = suspenseHandlerStackCursor.current, returnFiber !== null ? ((returnFiber.flags & 65536) === 0 && (returnFiber.flags |= 256), returnFiber.flags |= 65536, returnFiber.lanes = rootRenderLanes, value !== HydrationMismatchException && (root = Error(formatProdErrorMessage(422), { cause: value }), queueHydrationError(createCapturedValueAtFiber(root, sourceFiber)))) : (value !== HydrationMismatchException && (returnFiber = Error(formatProdErrorMessage(423), {
            cause: value
          }), queueHydrationError(createCapturedValueAtFiber(returnFiber, sourceFiber))), root = root.current.alternate, root.flags |= 65536, rootRenderLanes &= -rootRenderLanes, root.lanes |= rootRenderLanes, value = createCapturedValueAtFiber(value, sourceFiber), rootRenderLanes = createRootErrorUpdate(root.stateNode, value, rootRenderLanes), enqueueCapturedUpdate(root, rootRenderLanes), workInProgressRootExitStatus !== 4 && (workInProgressRootExitStatus = 2)), false;
        var wrapperError = Error(formatProdErrorMessage(520), { cause: value });
        wrapperError = createCapturedValueAtFiber(wrapperError, sourceFiber);
        workInProgressRootConcurrentErrors === null ? workInProgressRootConcurrentErrors = [wrapperError] : workInProgressRootConcurrentErrors.push(wrapperError);
        workInProgressRootExitStatus !== 4 && (workInProgressRootExitStatus = 2);
        if (returnFiber === null)
          return true;
        value = createCapturedValueAtFiber(value, sourceFiber);
        sourceFiber = returnFiber;
        do {
          switch (sourceFiber.tag) {
            case 3:
              return sourceFiber.flags |= 65536, root = rootRenderLanes & -rootRenderLanes, sourceFiber.lanes |= root, root = createRootErrorUpdate(sourceFiber.stateNode, value, root), enqueueCapturedUpdate(sourceFiber, root), false;
            case 1:
              returnFiber = sourceFiber.type;
              wrapperError = sourceFiber.stateNode;
              if ((sourceFiber.flags & 128) === 0 && (typeof returnFiber.getDerivedStateFromError === "function" || wrapperError !== null && typeof wrapperError.componentDidCatch === "function" && (legacyErrorBoundariesThatAlreadyFailed === null || !legacyErrorBoundariesThatAlreadyFailed.has(wrapperError))))
                return sourceFiber.flags |= 65536, rootRenderLanes &= -rootRenderLanes, sourceFiber.lanes |= rootRenderLanes, rootRenderLanes = createClassErrorUpdate(rootRenderLanes), initializeClassErrorUpdate(rootRenderLanes, root, sourceFiber, value), enqueueCapturedUpdate(sourceFiber, rootRenderLanes), false;
              break;
            case 22:
              if (sourceFiber.memoizedState !== null)
                return sourceFiber.flags |= 65536, false;
          }
          sourceFiber = sourceFiber.return;
        } while (sourceFiber !== null);
        return false;
      }
      function reconcileChildren(current, workInProgress, nextChildren, renderLanes) {
        workInProgress.child = current === null ? mountChildFibers(workInProgress, null, nextChildren, renderLanes) : reconcileChildFibers(workInProgress, current.child, nextChildren, renderLanes);
      }
      function updateForwardRef(current, workInProgress, Component, nextProps, renderLanes) {
        Component = Component.render;
        var ref = workInProgress.ref;
        if ("ref" in nextProps) {
          var propsWithoutRef = {};
          for (var key in nextProps)
            key !== "ref" && (propsWithoutRef[key] = nextProps[key]);
        } else
          propsWithoutRef = nextProps;
        prepareToReadContext(workInProgress);
        nextProps = renderWithHooks(current, workInProgress, Component, propsWithoutRef, ref, renderLanes);
        key = checkDidRenderIdHook();
        if (current !== null && !didReceiveUpdate)
          return bailoutHooks(current, workInProgress, renderLanes), bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
        isHydrating && key && pushMaterializedTreeId(workInProgress);
        workInProgress.flags |= 1;
        reconcileChildren(current, workInProgress, nextProps, renderLanes);
        return workInProgress.child;
      }
      function updateMemoComponent(current, workInProgress, Component, nextProps, renderLanes) {
        if (current === null) {
          var type = Component.type;
          if (typeof type === "function" && !shouldConstruct(type) && type.defaultProps === undefined && Component.compare === null)
            return workInProgress.tag = 15, workInProgress.type = type, updateSimpleMemoComponent(current, workInProgress, type, nextProps, renderLanes);
          current = createFiberFromTypeAndProps(Component.type, null, nextProps, workInProgress, workInProgress.mode, renderLanes);
          current.ref = workInProgress.ref;
          current.return = workInProgress;
          return workInProgress.child = current;
        }
        type = current.child;
        if (!checkScheduledUpdateOrContext(current, renderLanes)) {
          var prevProps = type.memoizedProps;
          Component = Component.compare;
          Component = Component !== null ? Component : shallowEqual;
          if (Component(prevProps, nextProps) && current.ref === workInProgress.ref)
            return bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
        }
        workInProgress.flags |= 1;
        current = createWorkInProgress(type, nextProps);
        current.ref = workInProgress.ref;
        current.return = workInProgress;
        return workInProgress.child = current;
      }
      function updateSimpleMemoComponent(current, workInProgress, Component, nextProps, renderLanes) {
        if (current !== null) {
          var prevProps = current.memoizedProps;
          if (shallowEqual(prevProps, nextProps) && current.ref === workInProgress.ref)
            if (didReceiveUpdate = false, workInProgress.pendingProps = nextProps = prevProps, checkScheduledUpdateOrContext(current, renderLanes))
              (current.flags & 131072) !== 0 && (didReceiveUpdate = true);
            else
              return workInProgress.lanes = current.lanes, bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
        }
        return updateFunctionComponent(current, workInProgress, Component, nextProps, renderLanes);
      }
      function updateOffscreenComponent(current, workInProgress, renderLanes, nextProps) {
        var nextChildren = nextProps.children, prevState = current !== null ? current.memoizedState : null;
        current === null && workInProgress.stateNode === null && (workInProgress.stateNode = {
          _visibility: 1,
          _pendingMarkers: null,
          _retryCache: null,
          _transitions: null
        });
        if (nextProps.mode === "hidden") {
          if ((workInProgress.flags & 128) !== 0) {
            prevState = prevState !== null ? prevState.baseLanes | renderLanes : renderLanes;
            if (current !== null) {
              nextProps = workInProgress.child = current.child;
              for (nextChildren = 0;nextProps !== null; )
                nextChildren = nextChildren | nextProps.lanes | nextProps.childLanes, nextProps = nextProps.sibling;
              nextProps = nextChildren & ~prevState;
            } else
              nextProps = 0, workInProgress.child = null;
            return deferHiddenOffscreenComponent(current, workInProgress, prevState, renderLanes, nextProps);
          }
          if ((renderLanes & 536870912) !== 0)
            workInProgress.memoizedState = { baseLanes: 0, cachePool: null }, current !== null && pushTransition(workInProgress, prevState !== null ? prevState.cachePool : null), prevState !== null ? pushHiddenContext(workInProgress, prevState) : reuseHiddenContextOnStack(), pushOffscreenSuspenseHandler(workInProgress);
          else
            return nextProps = workInProgress.lanes = 536870912, deferHiddenOffscreenComponent(current, workInProgress, prevState !== null ? prevState.baseLanes | renderLanes : renderLanes, renderLanes, nextProps);
        } else
          prevState !== null ? (pushTransition(workInProgress, prevState.cachePool), pushHiddenContext(workInProgress, prevState), reuseSuspenseHandlerOnStack(), workInProgress.memoizedState = null) : (current !== null && pushTransition(workInProgress, null), reuseHiddenContextOnStack(), reuseSuspenseHandlerOnStack());
        reconcileChildren(current, workInProgress, nextChildren, renderLanes);
        return workInProgress.child;
      }
      function bailoutOffscreenComponent(current, workInProgress) {
        current !== null && current.tag === 22 || workInProgress.stateNode !== null || (workInProgress.stateNode = {
          _visibility: 1,
          _pendingMarkers: null,
          _retryCache: null,
          _transitions: null
        });
        return workInProgress.sibling;
      }
      function deferHiddenOffscreenComponent(current, workInProgress, nextBaseLanes, renderLanes, remainingChildLanes) {
        var JSCompiler_inline_result = peekCacheFromPool();
        JSCompiler_inline_result = JSCompiler_inline_result === null ? null : {
          parent: isPrimaryRenderer ? CacheContext._currentValue : CacheContext._currentValue2,
          pool: JSCompiler_inline_result
        };
        workInProgress.memoizedState = {
          baseLanes: nextBaseLanes,
          cachePool: JSCompiler_inline_result
        };
        current !== null && pushTransition(workInProgress, null);
        reuseHiddenContextOnStack();
        pushOffscreenSuspenseHandler(workInProgress);
        current !== null && propagateParentContextChanges(current, workInProgress, renderLanes, true);
        workInProgress.childLanes = remainingChildLanes;
        return null;
      }
      function mountActivityChildren(workInProgress, nextProps) {
        nextProps = mountWorkInProgressOffscreenFiber({ mode: nextProps.mode, children: nextProps.children }, workInProgress.mode);
        nextProps.ref = workInProgress.ref;
        workInProgress.child = nextProps;
        nextProps.return = workInProgress;
        return nextProps;
      }
      function retryActivityComponentWithoutHydrating(current, workInProgress, renderLanes) {
        reconcileChildFibers(workInProgress, current.child, null, renderLanes);
        current = mountActivityChildren(workInProgress, workInProgress.pendingProps);
        current.flags |= 2;
        popSuspenseHandler(workInProgress);
        workInProgress.memoizedState = null;
        return current;
      }
      function updateActivityComponent(current, workInProgress, renderLanes) {
        var nextProps = workInProgress.pendingProps, didSuspend = (workInProgress.flags & 128) !== 0;
        workInProgress.flags &= -129;
        if (current === null) {
          if (isHydrating) {
            if (nextProps.mode === "hidden")
              return current = mountActivityChildren(workInProgress, nextProps), workInProgress.lanes = 536870912, current.memoizedState = { baseLanes: 0, cachePool: null }, bailoutOffscreenComponent(null, current);
            pushDehydratedActivitySuspenseHandler(workInProgress);
            (current = nextHydratableInstance) ? (current = canHydrateActivityInstance(current, rootOrSingletonContext), current !== null && (workInProgress.memoizedState = {
              dehydrated: current,
              treeContext: treeContextProvider !== null ? { id: treeContextId, overflow: treeContextOverflow } : null,
              retryLane: 536870912,
              hydrationErrors: null
            }, renderLanes = createFiberFromDehydratedFragment(current), renderLanes.return = workInProgress, workInProgress.child = renderLanes, hydrationParentFiber = workInProgress, nextHydratableInstance = null)) : current = null;
            if (current === null)
              throw throwOnHydrationMismatch(workInProgress);
            workInProgress.lanes = 536870912;
            return null;
          }
          return mountActivityChildren(workInProgress, nextProps);
        }
        var prevState = current.memoizedState;
        if (prevState !== null) {
          var dehydrated = prevState.dehydrated;
          pushDehydratedActivitySuspenseHandler(workInProgress);
          if (didSuspend)
            if (workInProgress.flags & 256)
              workInProgress.flags &= -257, workInProgress = retryActivityComponentWithoutHydrating(current, workInProgress, renderLanes);
            else if (workInProgress.memoizedState !== null)
              workInProgress.child = current.child, workInProgress.flags |= 128, workInProgress = null;
            else
              throw Error(formatProdErrorMessage(558));
          else if (didReceiveUpdate || propagateParentContextChanges(current, workInProgress, renderLanes, false), didSuspend = (renderLanes & current.childLanes) !== 0, didReceiveUpdate || didSuspend) {
            if (currentTreeHiddenStackCursor.current === null) {
              nextProps = workInProgressRoot;
              if (nextProps !== null && (dehydrated = getBumpedLaneForHydration(nextProps, renderLanes), dehydrated !== 0 && dehydrated !== prevState.retryLane))
                throw prevState.retryLane = dehydrated, enqueueConcurrentRenderForLane(current, dehydrated), scheduleUpdateOnFiber(nextProps, current, dehydrated), SelectiveHydrationException;
              renderDidSuspendDelayIfPossible();
            }
            workInProgress = retryActivityComponentWithoutHydrating(current, workInProgress, renderLanes);
          } else
            current = prevState.treeContext, supportsHydration && (nextHydratableInstance = getFirstHydratableChildWithinActivityInstance(dehydrated), hydrationParentFiber = workInProgress, isHydrating = true, hydrationErrors = null, rootOrSingletonContext = false, current !== null && restoreSuspendedTreeContext(workInProgress, current)), workInProgress = mountActivityChildren(workInProgress, nextProps), workInProgress.flags |= 134221824;
          return workInProgress;
        }
        current = createWorkInProgress(current.child, {
          mode: nextProps.mode,
          children: nextProps.children
        });
        current.ref = workInProgress.ref;
        workInProgress.child = current;
        current.return = workInProgress;
        return current;
      }
      function markRef(current, workInProgress) {
        var ref = workInProgress.ref;
        if (ref === null)
          current !== null && current.ref !== null && (workInProgress.flags |= 4194816);
        else {
          if (typeof ref !== "function" && typeof ref !== "object")
            throw Error(formatProdErrorMessage(284));
          if (current === null || current.ref !== ref)
            workInProgress.flags |= 4194816;
        }
      }
      function updateFunctionComponent(current, workInProgress, Component, nextProps, renderLanes) {
        prepareToReadContext(workInProgress);
        Component = renderWithHooks(current, workInProgress, Component, nextProps, undefined, renderLanes);
        nextProps = checkDidRenderIdHook();
        if (current !== null && !didReceiveUpdate)
          return bailoutHooks(current, workInProgress, renderLanes), bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
        isHydrating && nextProps && pushMaterializedTreeId(workInProgress);
        workInProgress.flags |= 1;
        reconcileChildren(current, workInProgress, Component, renderLanes);
        return workInProgress.child;
      }
      function replayFunctionComponent(current, workInProgress, nextProps, Component, secondArg, renderLanes) {
        prepareToReadContext(workInProgress);
        workInProgress.updateQueue = null;
        nextProps = renderWithHooksAgain(workInProgress, Component, nextProps, secondArg);
        finishRenderingHooks(current);
        Component = checkDidRenderIdHook();
        if (current !== null && !didReceiveUpdate)
          return bailoutHooks(current, workInProgress, renderLanes), bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
        isHydrating && Component && pushMaterializedTreeId(workInProgress);
        workInProgress.flags |= 1;
        reconcileChildren(current, workInProgress, nextProps, renderLanes);
        return workInProgress.child;
      }
      function updateClassComponent(current, workInProgress, Component, nextProps, renderLanes) {
        prepareToReadContext(workInProgress);
        if (workInProgress.stateNode === null) {
          var context = emptyContextObject, contextType = Component.contextType;
          typeof contextType === "object" && contextType !== null && (context = readContext(contextType));
          context = new Component(nextProps, context);
          workInProgress.memoizedState = context.state !== null && context.state !== undefined ? context.state : null;
          context.updater = classComponentUpdater;
          workInProgress.stateNode = context;
          context._reactInternals = workInProgress;
          context = workInProgress.stateNode;
          context.props = nextProps;
          context.state = workInProgress.memoizedState;
          context.refs = {};
          initializeUpdateQueue(workInProgress);
          contextType = Component.contextType;
          context.context = typeof contextType === "object" && contextType !== null ? readContext(contextType) : emptyContextObject;
          context.state = workInProgress.memoizedState;
          contextType = Component.getDerivedStateFromProps;
          typeof contextType === "function" && (applyDerivedStateFromProps(workInProgress, Component, contextType, nextProps), context.state = workInProgress.memoizedState);
          typeof Component.getDerivedStateFromProps === "function" || typeof context.getSnapshotBeforeUpdate === "function" || typeof context.UNSAFE_componentWillMount !== "function" && typeof context.componentWillMount !== "function" || (contextType = context.state, typeof context.componentWillMount === "function" && context.componentWillMount(), typeof context.UNSAFE_componentWillMount === "function" && context.UNSAFE_componentWillMount(), contextType !== context.state && classComponentUpdater.enqueueReplaceState(context, context.state, null), processUpdateQueue(workInProgress, nextProps, context, renderLanes), suspendIfUpdateReadFromEntangledAsyncAction(), context.state = workInProgress.memoizedState);
          typeof context.componentDidMount === "function" && (workInProgress.flags |= 4194308);
          nextProps = true;
        } else if (current === null) {
          context = workInProgress.stateNode;
          var unresolvedOldProps = workInProgress.memoizedProps, oldProps = resolveClassComponentProps(Component, unresolvedOldProps);
          context.props = oldProps;
          var oldContext = context.context, contextType$jscomp$0 = Component.contextType;
          contextType = emptyContextObject;
          typeof contextType$jscomp$0 === "object" && contextType$jscomp$0 !== null && (contextType = readContext(contextType$jscomp$0));
          var getDerivedStateFromProps = Component.getDerivedStateFromProps;
          contextType$jscomp$0 = typeof getDerivedStateFromProps === "function" || typeof context.getSnapshotBeforeUpdate === "function";
          unresolvedOldProps = workInProgress.pendingProps !== unresolvedOldProps;
          contextType$jscomp$0 || typeof context.UNSAFE_componentWillReceiveProps !== "function" && typeof context.componentWillReceiveProps !== "function" || (unresolvedOldProps || oldContext !== contextType) && callComponentWillReceiveProps(workInProgress, context, nextProps, contextType);
          hasForceUpdate = false;
          var oldState = workInProgress.memoizedState;
          context.state = oldState;
          processUpdateQueue(workInProgress, nextProps, context, renderLanes);
          suspendIfUpdateReadFromEntangledAsyncAction();
          oldContext = workInProgress.memoizedState;
          unresolvedOldProps || oldState !== oldContext || hasForceUpdate ? (typeof getDerivedStateFromProps === "function" && (applyDerivedStateFromProps(workInProgress, Component, getDerivedStateFromProps, nextProps), oldContext = workInProgress.memoizedState), (oldProps = hasForceUpdate || checkShouldComponentUpdate(workInProgress, Component, oldProps, nextProps, oldState, oldContext, contextType)) ? (contextType$jscomp$0 || typeof context.UNSAFE_componentWillMount !== "function" && typeof context.componentWillMount !== "function" || (typeof context.componentWillMount === "function" && context.componentWillMount(), typeof context.UNSAFE_componentWillMount === "function" && context.UNSAFE_componentWillMount()), typeof context.componentDidMount === "function" && (workInProgress.flags |= 4194308)) : (typeof context.componentDidMount === "function" && (workInProgress.flags |= 4194308), workInProgress.memoizedProps = nextProps, workInProgress.memoizedState = oldContext), context.props = nextProps, context.state = oldContext, context.context = contextType, nextProps = oldProps) : (typeof context.componentDidMount === "function" && (workInProgress.flags |= 4194308), nextProps = false);
        } else {
          context = workInProgress.stateNode;
          cloneUpdateQueue(current, workInProgress);
          contextType = workInProgress.memoizedProps;
          contextType$jscomp$0 = resolveClassComponentProps(Component, contextType);
          context.props = contextType$jscomp$0;
          getDerivedStateFromProps = workInProgress.pendingProps;
          oldState = context.context;
          oldContext = Component.contextType;
          oldProps = emptyContextObject;
          typeof oldContext === "object" && oldContext !== null && (oldProps = readContext(oldContext));
          unresolvedOldProps = Component.getDerivedStateFromProps;
          (oldContext = typeof unresolvedOldProps === "function" || typeof context.getSnapshotBeforeUpdate === "function") || typeof context.UNSAFE_componentWillReceiveProps !== "function" && typeof context.componentWillReceiveProps !== "function" || (contextType !== getDerivedStateFromProps || oldState !== oldProps) && callComponentWillReceiveProps(workInProgress, context, nextProps, oldProps);
          hasForceUpdate = false;
          oldState = workInProgress.memoizedState;
          context.state = oldState;
          processUpdateQueue(workInProgress, nextProps, context, renderLanes);
          suspendIfUpdateReadFromEntangledAsyncAction();
          var newState = workInProgress.memoizedState;
          contextType !== getDerivedStateFromProps || oldState !== newState || hasForceUpdate || current !== null && current.dependencies !== null && checkIfContextChanged(current.dependencies) ? (typeof unresolvedOldProps === "function" && (applyDerivedStateFromProps(workInProgress, Component, unresolvedOldProps, nextProps), newState = workInProgress.memoizedState), (contextType$jscomp$0 = hasForceUpdate || checkShouldComponentUpdate(workInProgress, Component, contextType$jscomp$0, nextProps, oldState, newState, oldProps) || current !== null && current.dependencies !== null && checkIfContextChanged(current.dependencies)) ? (oldContext || typeof context.UNSAFE_componentWillUpdate !== "function" && typeof context.componentWillUpdate !== "function" || (typeof context.componentWillUpdate === "function" && context.componentWillUpdate(nextProps, newState, oldProps), typeof context.UNSAFE_componentWillUpdate === "function" && context.UNSAFE_componentWillUpdate(nextProps, newState, oldProps)), typeof context.componentDidUpdate === "function" && (workInProgress.flags |= 4), typeof context.getSnapshotBeforeUpdate === "function" && (workInProgress.flags |= 1024)) : (typeof context.componentDidUpdate !== "function" || contextType === current.memoizedProps && oldState === current.memoizedState || (workInProgress.flags |= 4), typeof context.getSnapshotBeforeUpdate !== "function" || contextType === current.memoizedProps && oldState === current.memoizedState || (workInProgress.flags |= 1024), workInProgress.memoizedProps = nextProps, workInProgress.memoizedState = newState), context.props = nextProps, context.state = newState, context.context = oldProps, nextProps = contextType$jscomp$0) : (typeof context.componentDidUpdate !== "function" || contextType === current.memoizedProps && oldState === current.memoizedState || (workInProgress.flags |= 4), typeof context.getSnapshotBeforeUpdate !== "function" || contextType === current.memoizedProps && oldState === current.memoizedState || (workInProgress.flags |= 1024), nextProps = false);
        }
        context = nextProps;
        markRef(current, workInProgress);
        nextProps = (workInProgress.flags & 128) !== 0;
        context || nextProps ? (context = workInProgress.stateNode, Component = nextProps && typeof Component.getDerivedStateFromError !== "function" ? null : context.render(), workInProgress.flags |= 1, current !== null && nextProps ? (workInProgress.child = reconcileChildFibers(workInProgress, current.child, null, renderLanes), workInProgress.child = reconcileChildFibers(workInProgress, null, Component, renderLanes)) : reconcileChildren(current, workInProgress, Component, renderLanes), workInProgress.memoizedState = context.state, current = workInProgress.child) : current = bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
        return current;
      }
      function mountHostRootWithoutHydrating(current, workInProgress, nextChildren, renderLanes) {
        resetHydrationState();
        workInProgress.flags |= 256;
        reconcileChildren(current, workInProgress, nextChildren, renderLanes);
        return workInProgress.child;
      }
      function mountSuspenseOffscreenState(renderLanes) {
        return { baseLanes: renderLanes, cachePool: getSuspendedCache() };
      }
      function getRemainingWorkInPrimaryTree(current, primaryTreeDidDefer, renderLanes) {
        current = current !== null ? current.childLanes & ~renderLanes : 0;
        primaryTreeDidDefer && (current |= workInProgressDeferredLane);
        return current;
      }
      function updateSuspenseComponent(current, workInProgress, renderLanes) {
        var nextProps = workInProgress.pendingProps, showFallback = false, didSuspend = (workInProgress.flags & 128) !== 0, JSCompiler_temp;
        (JSCompiler_temp = didSuspend) || (JSCompiler_temp = current !== null && current.memoizedState === null ? false : (suspenseStackCursor.current & 2) !== 0);
        JSCompiler_temp && (showFallback = true, workInProgress.flags &= -129);
        JSCompiler_temp = (workInProgress.flags & 32) !== 0;
        workInProgress.flags &= -33;
        if (current === null) {
          if (isHydrating) {
            showFallback ? pushPrimaryTreeSuspenseHandler(workInProgress) : reuseSuspenseHandlerOnStack();
            (current = nextHydratableInstance) ? (current = canHydrateSuspenseInstance(current, rootOrSingletonContext), current !== null && (workInProgress.memoizedState = {
              dehydrated: current,
              treeContext: treeContextProvider !== null ? { id: treeContextId, overflow: treeContextOverflow } : null,
              retryLane: 536870912,
              hydrationErrors: null
            }, renderLanes = createFiberFromDehydratedFragment(current), renderLanes.return = workInProgress, workInProgress.child = renderLanes, hydrationParentFiber = workInProgress, nextHydratableInstance = null)) : current = null;
            if (current === null)
              throw throwOnHydrationMismatch(workInProgress);
            isSuspenseInstanceFallback(current) ? workInProgress.lanes = 32 : workInProgress.lanes = 536870912;
            return null;
          }
          didSuspend = nextProps.children;
          nextProps = nextProps.fallback;
          if (showFallback)
            return reuseSuspenseHandlerOnStack(), showFallback = workInProgress.mode, didSuspend = mountWorkInProgressOffscreenFiber({ mode: "hidden", children: didSuspend }, showFallback), nextProps = createFiberFromFragment(nextProps, showFallback, renderLanes, null), didSuspend.return = workInProgress, nextProps.return = workInProgress, didSuspend.sibling = nextProps, workInProgress.child = didSuspend, nextProps = workInProgress.child, nextProps.memoizedState = mountSuspenseOffscreenState(renderLanes), nextProps.childLanes = getRemainingWorkInPrimaryTree(current, JSCompiler_temp, renderLanes), workInProgress.memoizedState = SUSPENDED_MARKER, bailoutOffscreenComponent(null, nextProps);
          pushPrimaryTreeSuspenseHandler(workInProgress);
          return mountSuspensePrimaryChildren(workInProgress, didSuspend);
        }
        var prevState = current.memoizedState;
        if (prevState !== null) {
          var dehydrated$75 = prevState.dehydrated;
          if (dehydrated$75 !== null)
            return updateDehydratedSuspenseComponent(current, workInProgress, didSuspend, JSCompiler_temp, nextProps, dehydrated$75, prevState, renderLanes);
        }
        if (showFallback)
          return reuseSuspenseHandlerOnStack(), showFallback = nextProps.fallback, didSuspend = workInProgress.mode, prevState = current.child, dehydrated$75 = prevState.sibling, nextProps = createWorkInProgress(prevState, {
            mode: "hidden",
            children: nextProps.children
          }), nextProps.subtreeFlags = prevState.subtreeFlags & 1206910976, dehydrated$75 !== null ? showFallback = createWorkInProgress(dehydrated$75, showFallback) : (showFallback = createFiberFromFragment(showFallback, didSuspend, renderLanes, null), showFallback.flags |= 2), showFallback.return = workInProgress, nextProps.return = workInProgress, nextProps.sibling = showFallback, workInProgress.child = nextProps, bailoutOffscreenComponent(null, nextProps), nextProps = workInProgress.child, showFallback = current.child.memoizedState, showFallback === null ? showFallback = mountSuspenseOffscreenState(renderLanes) : (didSuspend = showFallback.cachePool, didSuspend !== null ? (prevState = isPrimaryRenderer ? CacheContext._currentValue : CacheContext._currentValue2, didSuspend = didSuspend.parent !== prevState ? { parent: prevState, pool: prevState } : didSuspend) : didSuspend = getSuspendedCache(), showFallback = {
            baseLanes: showFallback.baseLanes | renderLanes,
            cachePool: didSuspend
          }), nextProps.memoizedState = showFallback, nextProps.childLanes = getRemainingWorkInPrimaryTree(current, JSCompiler_temp, renderLanes), workInProgress.memoizedState = SUSPENDED_MARKER, bailoutOffscreenComponent(current.child, nextProps);
        pushPrimaryTreeSuspenseHandler(workInProgress);
        renderLanes = current.child;
        current = renderLanes.sibling;
        renderLanes = createWorkInProgress(renderLanes, {
          mode: "visible",
          children: nextProps.children
        });
        renderLanes.return = workInProgress;
        renderLanes.sibling = null;
        current !== null && (JSCompiler_temp = workInProgress.deletions, JSCompiler_temp === null ? (workInProgress.deletions = [current], workInProgress.flags |= 16) : JSCompiler_temp.push(current));
        workInProgress.child = renderLanes;
        workInProgress.memoizedState = null;
        return renderLanes;
      }
      function mountSuspensePrimaryChildren(workInProgress, primaryChildren) {
        primaryChildren = mountWorkInProgressOffscreenFiber({ mode: "visible", children: primaryChildren }, workInProgress.mode);
        primaryChildren.return = workInProgress;
        return workInProgress.child = primaryChildren;
      }
      function mountWorkInProgressOffscreenFiber(offscreenProps, mode) {
        offscreenProps = createFiber(22, offscreenProps, null, mode);
        offscreenProps.lanes = 0;
        return offscreenProps;
      }
      function retrySuspenseComponentWithoutHydrating(current, workInProgress, renderLanes) {
        reconcileChildFibers(workInProgress, current.child, null, renderLanes);
        current = mountSuspensePrimaryChildren(workInProgress, workInProgress.pendingProps.children);
        current.flags |= 2;
        workInProgress.memoizedState = null;
        return current;
      }
      function updateDehydratedSuspenseComponent(current, workInProgress, didSuspend, didPrimaryChildrenDefer, nextProps, suspenseInstance, suspenseState, renderLanes) {
        if (didSuspend) {
          if (workInProgress.flags & 256)
            return pushPrimaryTreeSuspenseHandler(workInProgress), workInProgress.flags &= -257, retrySuspenseComponentWithoutHydrating(current, workInProgress, renderLanes);
          if (workInProgress.memoizedState !== null)
            return reuseSuspenseHandlerOnStack(), workInProgress.child = current.child, workInProgress.flags |= 128, null;
          reuseSuspenseHandlerOnStack();
          suspenseInstance = nextProps.fallback;
          suspenseState = workInProgress.mode;
          nextProps = mountWorkInProgressOffscreenFiber({ mode: "visible", children: nextProps.children }, suspenseState);
          suspenseInstance = createFiberFromFragment(suspenseInstance, suspenseState, renderLanes, null);
          suspenseInstance.flags |= 2;
          nextProps.return = workInProgress;
          suspenseInstance.return = workInProgress;
          nextProps.sibling = suspenseInstance;
          workInProgress.child = nextProps;
          reconcileChildFibers(workInProgress, current.child, null, renderLanes);
          nextProps = workInProgress.child;
          nextProps.memoizedState = mountSuspenseOffscreenState(renderLanes);
          nextProps.childLanes = getRemainingWorkInPrimaryTree(current, didPrimaryChildrenDefer, renderLanes);
          workInProgress.memoizedState = SUSPENDED_MARKER;
          return bailoutOffscreenComponent(null, nextProps);
        }
        pushPrimaryTreeSuspenseHandler(workInProgress);
        if (isSuspenseInstanceFallback(suspenseInstance))
          return didPrimaryChildrenDefer = getSuspenseInstanceFallbackErrorDetails(suspenseInstance).digest, didPrimaryChildrenDefer !== "" && (nextProps = Error(formatProdErrorMessage(419)), nextProps.stack = "", nextProps.digest = didPrimaryChildrenDefer, queueHydrationError({ value: nextProps, source: null, stack: null })), retrySuspenseComponentWithoutHydrating(current, workInProgress, renderLanes);
        didReceiveUpdate || propagateParentContextChanges(current, workInProgress, renderLanes, false);
        didPrimaryChildrenDefer = (renderLanes & current.childLanes) !== 0;
        if (didReceiveUpdate || didPrimaryChildrenDefer) {
          if (currentTreeHiddenStackCursor.current !== null)
            return retrySuspenseComponentWithoutHydrating(current, workInProgress, renderLanes);
          didPrimaryChildrenDefer = workInProgressRoot;
          if (didPrimaryChildrenDefer !== null && (nextProps = getBumpedLaneForHydration(didPrimaryChildrenDefer, renderLanes), nextProps !== 0 && nextProps !== suspenseState.retryLane))
            throw suspenseState.retryLane = nextProps, enqueueConcurrentRenderForLane(current, nextProps), scheduleUpdateOnFiber(didPrimaryChildrenDefer, current, nextProps), SelectiveHydrationException;
          isSuspenseInstancePending(suspenseInstance) || renderDidSuspendDelayIfPossible();
          return retrySuspenseComponentWithoutHydrating(current, workInProgress, renderLanes);
        }
        if (isSuspenseInstancePending(suspenseInstance))
          return workInProgress.flags |= 192, workInProgress.child = current.child, null;
        current = suspenseState.treeContext;
        supportsHydration && (nextHydratableInstance = getFirstHydratableChildWithinSuspenseInstance(suspenseInstance), hydrationParentFiber = workInProgress, isHydrating = true, hydrationErrors = null, rootOrSingletonContext = false, current !== null && restoreSuspendedTreeContext(workInProgress, current));
        workInProgress = mountSuspensePrimaryChildren(workInProgress, nextProps.children);
        workInProgress.flags |= 134221824;
        return workInProgress;
      }
      function scheduleSuspenseWorkOnFiber(fiber, renderLanes, propagationRoot) {
        fiber.lanes |= renderLanes;
        var alternate = fiber.alternate;
        alternate !== null && (alternate.lanes |= renderLanes);
        scheduleContextWorkOnParentPath(fiber.return, renderLanes, propagationRoot);
      }
      function findLastContentRow(firstChild) {
        for (var lastContentRow = null;firstChild !== null; ) {
          var currentRow = firstChild.alternate;
          currentRow !== null && findFirstSuspended(currentRow) === null && (lastContentRow = firstChild);
          firstChild = firstChild.sibling;
        }
        return lastContentRow;
      }
      function initSuspenseListRenderState(workInProgress, isBackwards, tail, lastContentRow, tailMode, treeForkCount) {
        var renderState = workInProgress.memoizedState;
        renderState === null ? workInProgress.memoizedState = {
          isBackwards,
          rendering: null,
          renderingStartTime: 0,
          last: lastContentRow,
          tail,
          tailMode,
          treeForkCount
        } : (renderState.isBackwards = isBackwards, renderState.rendering = null, renderState.renderingStartTime = 0, renderState.last = lastContentRow, renderState.tail = tail, renderState.tailMode = tailMode, renderState.treeForkCount = treeForkCount);
      }
      function reverseChildren(fiber) {
        var row = fiber.child;
        for (fiber.child = null;row !== null; ) {
          var nextRow = row.sibling;
          row.sibling = fiber.child;
          fiber.child = row;
          row = nextRow;
        }
      }
      function updateSuspenseListComponent(current, workInProgress, renderLanes) {
        var nextProps = workInProgress.pendingProps, revealOrder = nextProps.revealOrder, tailMode = nextProps.tail;
        nextProps = nextProps.children;
        var suspenseContext = suspenseStackCursor.current;
        if (workInProgress.flags & 128)
          return pushSuspenseListContext(workInProgress, suspenseContext), null;
        var shouldForceFallback = (suspenseContext & 2) !== 0;
        shouldForceFallback ? (suspenseContext = suspenseContext & 1 | 2, workInProgress.flags |= 128) : suspenseContext &= 1;
        pushSuspenseListContext(workInProgress, suspenseContext);
        revealOrder === "backwards" && current !== null ? (reverseChildren(current), reconcileChildren(current, workInProgress, nextProps, renderLanes), reverseChildren(current)) : reconcileChildren(current, workInProgress, nextProps, renderLanes);
        nextProps = isHydrating ? treeForkCount : 0;
        if (!shouldForceFallback && current !== null && (current.flags & 128) !== 0)
          a:
            for (current = workInProgress.child;current !== null; ) {
              if (current.tag === 13)
                current.memoizedState !== null && scheduleSuspenseWorkOnFiber(current, renderLanes, workInProgress);
              else if (current.tag === 19)
                scheduleSuspenseWorkOnFiber(current, renderLanes, workInProgress);
              else if (current.child !== null) {
                current.child.return = current;
                current = current.child;
                continue;
              }
              if (current === workInProgress)
                break a;
              for (;current.sibling === null; ) {
                if (current.return === null || current.return === workInProgress)
                  break a;
                current = current.return;
              }
              current.sibling.return = current.return;
              current = current.sibling;
            }
        switch (revealOrder) {
          case "backwards":
            renderLanes = findLastContentRow(workInProgress.child);
            renderLanes === null ? (revealOrder = workInProgress.child, workInProgress.child = null) : (revealOrder = renderLanes.sibling, renderLanes.sibling = null, reverseChildren(workInProgress));
            initSuspenseListRenderState(workInProgress, true, revealOrder, null, tailMode, nextProps);
            break;
          case "unstable_legacy-backwards":
            renderLanes = null;
            revealOrder = workInProgress.child;
            for (workInProgress.child = null;revealOrder !== null; ) {
              current = revealOrder.alternate;
              if (current !== null && findFirstSuspended(current) === null) {
                workInProgress.child = revealOrder;
                break;
              }
              current = revealOrder.sibling;
              revealOrder.sibling = renderLanes;
              renderLanes = revealOrder;
              revealOrder = current;
            }
            initSuspenseListRenderState(workInProgress, true, renderLanes, null, tailMode, nextProps);
            break;
          case "together":
            initSuspenseListRenderState(workInProgress, false, null, null, undefined, nextProps);
            break;
          case "independent":
            workInProgress.memoizedState = null;
            break;
          default:
            renderLanes = findLastContentRow(workInProgress.child), renderLanes === null ? (revealOrder = workInProgress.child, workInProgress.child = null) : (revealOrder = renderLanes.sibling, renderLanes.sibling = null), initSuspenseListRenderState(workInProgress, false, revealOrder, renderLanes, tailMode, nextProps);
        }
        return workInProgress.child;
      }
      function updateContextProvider(current, workInProgress, renderLanes) {
        var newProps = workInProgress.pendingProps;
        pushProvider(workInProgress, workInProgress.type, newProps.value);
        reconcileChildren(current, workInProgress, newProps.children, renderLanes);
        return workInProgress.child;
      }
      function bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes) {
        current !== null && (workInProgress.dependencies = current.dependencies);
        workInProgressRootSkippedLanes |= workInProgress.lanes;
        if ((renderLanes & workInProgress.childLanes) === 0)
          if (current !== null) {
            if (propagateParentContextChanges(current, workInProgress, renderLanes, false), (renderLanes & workInProgress.childLanes) === 0)
              return null;
          } else
            return null;
        if (current !== null && workInProgress.child !== current.child)
          throw Error(formatProdErrorMessage(153));
        if (workInProgress.child !== null) {
          current = workInProgress.child;
          renderLanes = createWorkInProgress(current, current.pendingProps);
          workInProgress.child = renderLanes;
          for (renderLanes.return = workInProgress;current.sibling !== null; )
            current = current.sibling, renderLanes = renderLanes.sibling = createWorkInProgress(current, current.pendingProps), renderLanes.return = workInProgress;
          renderLanes.sibling = null;
        }
        return workInProgress.child;
      }
      function checkScheduledUpdateOrContext(current, renderLanes) {
        if ((current.lanes & renderLanes) !== 0)
          return true;
        current = current.dependencies;
        return current !== null && checkIfContextChanged(current) ? true : false;
      }
      function attemptEarlyBailoutIfNoScheduledUpdate(current, workInProgress, renderLanes) {
        switch (workInProgress.tag) {
          case 3:
            pushHostContainer(workInProgress, workInProgress.stateNode.containerInfo);
            pushProvider(workInProgress, CacheContext, current.memoizedState.cache);
            resetHydrationState();
            break;
          case 27:
          case 5:
            pushHostContext(workInProgress);
            break;
          case 4:
            pushHostContainer(workInProgress, workInProgress.stateNode.containerInfo);
            break;
          case 10:
            pushProvider(workInProgress, workInProgress.type, workInProgress.memoizedProps.value);
            break;
          case 31:
            if (workInProgress.memoizedState !== null)
              return workInProgress.flags |= 128, pushDehydratedActivitySuspenseHandler(workInProgress), null;
            break;
          case 13:
            var state$86 = workInProgress.memoizedState;
            if (state$86 !== null) {
              if (state$86.dehydrated !== null)
                return pushPrimaryTreeSuspenseHandler(workInProgress), workInProgress.flags |= 128, null;
              state$86 = propagateParentContextChanges(current, workInProgress, renderLanes, false);
              var primaryChildLanes = workInProgress.child.childLanes;
              if (state$86 || (renderLanes & primaryChildLanes) !== 0)
                return updateSuspenseComponent(current, workInProgress, renderLanes);
              pushPrimaryTreeSuspenseHandler(workInProgress);
              current = bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
              return current !== null ? current.sibling : null;
            }
            pushPrimaryTreeSuspenseHandler(workInProgress);
            break;
          case 19:
            if (workInProgress.flags & 128)
              return updateSuspenseListComponent(current, workInProgress, renderLanes);
            primaryChildLanes = (current.flags & 128) !== 0;
            state$86 = (renderLanes & workInProgress.childLanes) !== 0;
            state$86 || (propagateParentContextChanges(current, workInProgress, renderLanes, false), state$86 = (renderLanes & workInProgress.childLanes) !== 0);
            if (primaryChildLanes) {
              if (state$86)
                return updateSuspenseListComponent(current, workInProgress, renderLanes);
              workInProgress.flags |= 128;
            }
            primaryChildLanes = workInProgress.memoizedState;
            primaryChildLanes !== null && (primaryChildLanes.rendering = null, primaryChildLanes.tail = null, primaryChildLanes.lastEffect = null);
            pushSuspenseListContext(workInProgress, suspenseStackCursor.current);
            if (state$86)
              break;
            else
              return null;
          case 22:
            return workInProgress.lanes = 0, updateOffscreenComponent(current, workInProgress, renderLanes, workInProgress.pendingProps);
          case 24:
            pushProvider(workInProgress, CacheContext, current.memoizedState.cache);
        }
        return bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
      }
      function beginWork(current, workInProgress, renderLanes) {
        if (current !== null)
          if (current.memoizedProps !== workInProgress.pendingProps)
            didReceiveUpdate = true;
          else {
            if (!checkScheduledUpdateOrContext(current, renderLanes) && (workInProgress.flags & 128) === 0)
              return didReceiveUpdate = false, attemptEarlyBailoutIfNoScheduledUpdate(current, workInProgress, renderLanes);
            didReceiveUpdate = (current.flags & 131072) !== 0 ? true : false;
          }
        else
          didReceiveUpdate = false, isHydrating && (workInProgress.flags & 1048576) !== 0 && pushTreeId(workInProgress, treeForkCount, workInProgress.index);
        workInProgress.lanes = 0;
        switch (workInProgress.tag) {
          case 16:
            a: {
              var props = workInProgress.pendingProps;
              current = resolveLazy(workInProgress.elementType);
              workInProgress.type = current;
              if (typeof current === "function")
                shouldConstruct(current) ? (props = resolveClassComponentProps(current, props), workInProgress.tag = 1, workInProgress = updateClassComponent(null, workInProgress, current, props, renderLanes)) : (workInProgress.tag = 0, workInProgress = updateFunctionComponent(null, workInProgress, current, props, renderLanes));
              else {
                if (current !== undefined && current !== null) {
                  var $$typeof = current.$$typeof;
                  if ($$typeof === REACT_FORWARD_REF_TYPE) {
                    workInProgress.tag = 11;
                    workInProgress = updateForwardRef(null, workInProgress, current, props, renderLanes);
                    break a;
                  } else if ($$typeof === REACT_MEMO_TYPE) {
                    workInProgress.tag = 14;
                    workInProgress = updateMemoComponent(null, workInProgress, current, props, renderLanes);
                    break a;
                  } else if ($$typeof === REACT_CONTEXT_TYPE) {
                    workInProgress.tag = 10;
                    workInProgress.type = current;
                    workInProgress = updateContextProvider(null, workInProgress, renderLanes);
                    break a;
                  }
                }
                workInProgress = getComponentNameFromType(current) || current;
                throw Error(formatProdErrorMessage(306, workInProgress, ""));
              }
            }
            return workInProgress;
          case 0:
            return updateFunctionComponent(current, workInProgress, workInProgress.type, workInProgress.pendingProps, renderLanes);
          case 1:
            return props = workInProgress.type, $$typeof = resolveClassComponentProps(props, workInProgress.pendingProps), updateClassComponent(current, workInProgress, props, $$typeof, renderLanes);
          case 3:
            a: {
              pushHostContainer(workInProgress, workInProgress.stateNode.containerInfo);
              if (current === null)
                throw Error(formatProdErrorMessage(387));
              var nextProps = workInProgress.pendingProps;
              $$typeof = workInProgress.memoizedState;
              props = $$typeof.element;
              cloneUpdateQueue(current, workInProgress);
              processUpdateQueue(workInProgress, nextProps, null, renderLanes);
              var nextState = workInProgress.memoizedState;
              nextProps = nextState.cache;
              pushProvider(workInProgress, CacheContext, nextProps);
              nextProps !== $$typeof.cache && propagateContextChanges(workInProgress, [CacheContext], renderLanes, true);
              suspendIfUpdateReadFromEntangledAsyncAction();
              nextProps = nextState.element;
              if (supportsHydration && $$typeof.isDehydrated)
                if ($$typeof = {
                  element: nextProps,
                  isDehydrated: false,
                  cache: nextState.cache
                }, workInProgress.updateQueue.baseState = $$typeof, workInProgress.memoizedState = $$typeof, workInProgress.flags & 256) {
                  workInProgress = mountHostRootWithoutHydrating(current, workInProgress, nextProps, renderLanes);
                  break a;
                } else if (nextProps !== props) {
                  props = createCapturedValueAtFiber(Error(formatProdErrorMessage(424)), workInProgress);
                  queueHydrationError(props);
                  workInProgress = mountHostRootWithoutHydrating(current, workInProgress, nextProps, renderLanes);
                  break a;
                } else
                  for (supportsHydration && (nextHydratableInstance = getFirstHydratableChildWithinContainer(workInProgress.stateNode.containerInfo), hydrationParentFiber = workInProgress, isHydrating = true, hydrationErrors = null, rootOrSingletonContext = true), renderLanes = mountChildFibers(workInProgress, null, nextProps, renderLanes), workInProgress.child = renderLanes;renderLanes; )
                    renderLanes.flags = renderLanes.flags & -3 | 134221824, renderLanes = renderLanes.sibling;
              else {
                resetHydrationState();
                if (nextProps === props) {
                  workInProgress = bailoutOnAlreadyFinishedWork(current, workInProgress, renderLanes);
                  break a;
                }
                reconcileChildren(current, workInProgress, nextProps, renderLanes);
              }
              workInProgress = workInProgress.child;
            }
            return workInProgress;
          case 26:
            if (supportsResources)
              return markRef(current, workInProgress), current === null ? (renderLanes = getResource(workInProgress.type, null, workInProgress.pendingProps, null)) ? workInProgress.memoizedState = renderLanes : isHydrating || (workInProgress.stateNode = createHoistableInstance(workInProgress.type, workInProgress.pendingProps, rootInstanceStackCursor.current, workInProgress)) : workInProgress.memoizedState = getResource(workInProgress.type, current.memoizedProps, workInProgress.pendingProps, current.memoizedState), null;
          case 27:
            if (supportsSingletons)
              return pushHostContext(workInProgress), current === null && supportsSingletons && isHydrating && (props = workInProgress.stateNode = resolveSingletonInstance(workInProgress.type, workInProgress.pendingProps, rootInstanceStackCursor.current, contextStackCursor.current, false), hydrationParentFiber = workInProgress, rootOrSingletonContext = true, nextHydratableInstance = getFirstHydratableChildWithinSingleton(workInProgress.type, props, nextHydratableInstance)), reconcileChildren(current, workInProgress, workInProgress.pendingProps.children, renderLanes), markRef(current, workInProgress), current === null && (workInProgress.flags |= 4194304), workInProgress.child;
          case 5:
            if (current === null && isHydrating) {
              validateHydratableInstance(workInProgress.type, workInProgress.pendingProps, contextStackCursor.current);
              if ($$typeof = props = nextHydratableInstance)
                props = canHydrateInstance(props, workInProgress.type, workInProgress.pendingProps, rootOrSingletonContext), props !== null ? (workInProgress.stateNode = props, hydrationParentFiber = workInProgress, nextHydratableInstance = getFirstHydratableChild(props), rootOrSingletonContext = false, $$typeof = true) : $$typeof = false;
              $$typeof || throwOnHydrationMismatch(workInProgress);
            }
            pushHostContext(workInProgress);
            $$typeof = workInProgress.type;
            nextProps = workInProgress.pendingProps;
            nextState = current !== null ? current.memoizedProps : null;
            props = nextProps.children;
            shouldSetTextContent($$typeof, nextProps) ? props = null : nextState !== null && shouldSetTextContent($$typeof, nextState) && (workInProgress.flags |= 32);
            workInProgress.memoizedState !== null && ($$typeof = renderWithHooks(current, workInProgress, TransitionAwareHostComponent, null, null, renderLanes), isPrimaryRenderer ? HostTransitionContext._currentValue = $$typeof : HostTransitionContext._currentValue2 = $$typeof);
            markRef(current, workInProgress);
            reconcileChildren(current, workInProgress, props, renderLanes);
            return workInProgress.child;
          case 6:
            if (current === null && isHydrating) {
              validateHydratableTextInstance(workInProgress.pendingProps, contextStackCursor.current);
              if (current = renderLanes = nextHydratableInstance)
                renderLanes = canHydrateTextInstance(renderLanes, workInProgress.pendingProps, rootOrSingletonContext), renderLanes !== null ? (workInProgress.stateNode = renderLanes, hydrationParentFiber = workInProgress, nextHydratableInstance = null, current = true) : current = false;
              current || throwOnHydrationMismatch(workInProgress);
            }
            return null;
          case 13:
            return updateSuspenseComponent(current, workInProgress, renderLanes);
          case 4:
            return pushHostContainer(workInProgress, workInProgress.stateNode.containerInfo), props = workInProgress.pendingProps, current === null ? workInProgress.child = reconcileChildFibers(workInProgress, null, props, renderLanes) : reconcileChildren(current, workInProgress, props, renderLanes), workInProgress.child;
          case 11:
            return updateForwardRef(current, workInProgress, workInProgress.type, workInProgress.pendingProps, renderLanes);
          case 7:
            return props = workInProgress.pendingProps, markRef(current, workInProgress), reconcileChildren(current, workInProgress, props, renderLanes), workInProgress.child;
          case 8:
            return reconcileChildren(current, workInProgress, workInProgress.pendingProps.children, renderLanes), workInProgress.child;
          case 12:
            return reconcileChildren(current, workInProgress, workInProgress.pendingProps.children, renderLanes), workInProgress.child;
          case 10:
            return updateContextProvider(current, workInProgress, renderLanes);
          case 9:
            return $$typeof = workInProgress.type._context, props = workInProgress.pendingProps.children, prepareToReadContext(workInProgress), $$typeof = readContext($$typeof), props = props($$typeof), workInProgress.flags |= 1, reconcileChildren(current, workInProgress, props, renderLanes), workInProgress.child;
          case 14:
            return updateMemoComponent(current, workInProgress, workInProgress.type, workInProgress.pendingProps, renderLanes);
          case 15:
            return updateSimpleMemoComponent(current, workInProgress, workInProgress.type, workInProgress.pendingProps, renderLanes);
          case 19:
            return updateSuspenseListComponent(current, workInProgress, renderLanes);
          case 31:
            return updateActivityComponent(current, workInProgress, renderLanes);
          case 22:
            return updateOffscreenComponent(current, workInProgress, renderLanes, workInProgress.pendingProps);
          case 24:
            return prepareToReadContext(workInProgress), props = readContext(CacheContext), current === null ? ($$typeof = peekCacheFromPool(), $$typeof === null && ($$typeof = workInProgressRoot, nextProps = createCache(), $$typeof.pooledCache = nextProps, nextProps.refCount++, nextProps !== null && ($$typeof.pooledCacheLanes |= renderLanes), $$typeof = nextProps), workInProgress.memoizedState = {
              parent: props,
              cache: $$typeof
            }, initializeUpdateQueue(workInProgress), pushProvider(workInProgress, CacheContext, $$typeof)) : ((current.lanes & renderLanes) !== 0 && (cloneUpdateQueue(current, workInProgress), processUpdateQueue(workInProgress, null, null, renderLanes), suspendIfUpdateReadFromEntangledAsyncAction()), $$typeof = current.memoizedState, nextProps = workInProgress.memoizedState, $$typeof.parent !== props ? ($$typeof = { parent: props, cache: props }, workInProgress.memoizedState = $$typeof, workInProgress.lanes === 0 && (workInProgress.memoizedState = workInProgress.updateQueue.baseState = $$typeof), pushProvider(workInProgress, CacheContext, props)) : (props = nextProps.cache, pushProvider(workInProgress, CacheContext, props), props !== $$typeof.cache && propagateContextChanges(workInProgress, [CacheContext], renderLanes, true))), reconcileChildren(current, workInProgress, workInProgress.pendingProps.children, renderLanes), workInProgress.child;
          case 30:
            return workInProgress.stateNode === null && (workInProgress.stateNode = {
              autoName: null,
              paired: null,
              clones: null,
              ref: null
            }), props = workInProgress.pendingProps, props.name != null && props.name !== "auto" ? workInProgress.flags |= current === null ? 18882560 : 18874368 : isHydrating && pushMaterializedTreeId(workInProgress), current !== null && current.memoizedProps.name !== props.name ? workInProgress.flags |= 4194816 : markRef(current, workInProgress), reconcileChildren(current, workInProgress, props.children, renderLanes), workInProgress.child;
          case 29:
            throw workInProgress.pendingProps;
        }
        throw Error(formatProdErrorMessage(156, workInProgress.tag));
      }
      function markUpdate(workInProgress) {
        workInProgress.flags |= 4;
      }
      function markCloned(workInProgress) {
        supportsPersistence && (workInProgress.flags |= 8);
      }
      function doesRequireClone(current, completedWork) {
        if (current !== null && current.child === completedWork.child)
          return false;
        if ((completedWork.flags & 16) !== 0)
          return true;
        for (current = completedWork.child;current !== null; ) {
          if ((current.flags & 8218) !== 0 || (current.subtreeFlags & 8218) !== 0)
            return true;
          current = current.sibling;
        }
        return false;
      }
      function appendAllChildren(parent, workInProgress, needsVisibilityToggle, isHidden) {
        if (supportsMutation)
          for (needsVisibilityToggle = workInProgress.child;needsVisibilityToggle !== null; ) {
            if (needsVisibilityToggle.tag === 5 || needsVisibilityToggle.tag === 6)
              appendInitialChild(parent, needsVisibilityToggle.stateNode);
            else if (!(needsVisibilityToggle.tag === 4 || supportsSingletons && needsVisibilityToggle.tag === 27) && needsVisibilityToggle.child !== null) {
              needsVisibilityToggle.child.return = needsVisibilityToggle;
              needsVisibilityToggle = needsVisibilityToggle.child;
              continue;
            }
            if (needsVisibilityToggle === workInProgress)
              break;
            for (;needsVisibilityToggle.sibling === null; ) {
              if (needsVisibilityToggle.return === null || needsVisibilityToggle.return === workInProgress)
                return;
              needsVisibilityToggle = needsVisibilityToggle.return;
            }
            needsVisibilityToggle.sibling.return = needsVisibilityToggle.return;
            needsVisibilityToggle = needsVisibilityToggle.sibling;
          }
        else if (supportsPersistence)
          for (var node$89 = workInProgress.child;node$89 !== null; ) {
            if (node$89.tag === 5) {
              var instance = node$89.stateNode;
              needsVisibilityToggle && isHidden && (instance = cloneHiddenInstance(instance, node$89.type, node$89.memoizedProps));
              appendInitialChild(parent, instance);
            } else if (node$89.tag === 6)
              instance = node$89.stateNode, needsVisibilityToggle && isHidden && (instance = cloneHiddenTextInstance(instance, node$89.memoizedProps)), appendInitialChild(parent, instance);
            else if (node$89.tag !== 4) {
              if (node$89.tag === 22 && node$89.memoizedState !== null)
                instance = node$89.child, instance !== null && (instance.return = node$89), appendAllChildren(parent, node$89, true, true);
              else if (node$89.child !== null) {
                node$89.child.return = node$89;
                node$89 = node$89.child;
                continue;
              }
            }
            if (node$89 === workInProgress)
              break;
            for (;node$89.sibling === null; ) {
              if (node$89.return === null || node$89.return === workInProgress)
                return;
              node$89 = node$89.return;
            }
            node$89.sibling.return = node$89.return;
            node$89 = node$89.sibling;
          }
      }
      function appendAllChildrenToContainer(containerChildSet, workInProgress, needsVisibilityToggle, isHidden) {
        var hasOffscreenComponentChild = false;
        if (supportsPersistence)
          for (var node = workInProgress.child;node !== null; ) {
            if (node.tag === 5) {
              var instance = node.stateNode;
              needsVisibilityToggle && isHidden && (instance = cloneHiddenInstance(instance, node.type, node.memoizedProps));
              appendChildToContainerChildSet(containerChildSet, instance);
            } else if (node.tag === 6)
              instance = node.stateNode, needsVisibilityToggle && isHidden && (instance = cloneHiddenTextInstance(instance, node.memoizedProps)), appendChildToContainerChildSet(containerChildSet, instance);
            else if (node.tag !== 4) {
              if (node.tag === 22 && node.memoizedState !== null)
                hasOffscreenComponentChild = node.child, hasOffscreenComponentChild !== null && (hasOffscreenComponentChild.return = node), appendAllChildrenToContainer(containerChildSet, node, true, true), hasOffscreenComponentChild = true;
              else if (node.child !== null) {
                node.child.return = node;
                node = node.child;
                continue;
              }
            }
            if (node === workInProgress)
              break;
            for (;node.sibling === null; ) {
              if (node.return === null || node.return === workInProgress)
                return hasOffscreenComponentChild;
              node = node.return;
            }
            node.sibling.return = node.return;
            node = node.sibling;
          }
        return hasOffscreenComponentChild;
      }
      function updateHostContainer(current, workInProgress) {
        if (supportsPersistence && doesRequireClone(current, workInProgress)) {
          current = workInProgress.stateNode;
          var container = current.containerInfo, newChildSet = createContainerChildSet();
          appendAllChildrenToContainer(newChildSet, workInProgress, false, false);
          current.pendingChildren = newChildSet;
          markUpdate(workInProgress);
          finalizeContainerChildren(container, newChildSet);
        }
      }
      function updateHostComponent(current, workInProgress, type, newProps) {
        if (supportsMutation)
          current.memoizedProps !== newProps && markUpdate(workInProgress);
        else if (supportsPersistence) {
          var currentInstance = current.stateNode, oldProps$92 = current.memoizedProps;
          if ((current = doesRequireClone(current, workInProgress)) || oldProps$92 !== newProps) {
            var currentHostContext = contextStackCursor.current;
            oldProps$92 = cloneInstance(currentInstance, type, oldProps$92, newProps, !current, null);
            oldProps$92 === currentInstance ? workInProgress.stateNode = currentInstance : (markCloned(workInProgress), finalizeInitialChildren(oldProps$92, type, newProps, currentHostContext) && markUpdate(workInProgress), workInProgress.stateNode = oldProps$92, current && appendAllChildren(oldProps$92, workInProgress, false, false));
          } else
            workInProgress.stateNode = currentInstance;
        }
      }
      function preloadInstanceAndSuspendIfNeeded(workInProgress, type, oldProps, newProps, renderLanes) {
        if ((workInProgress.mode & 32) !== 0 && (oldProps === null ? maySuspendCommit(type, newProps) : maySuspendCommitOnUpdate(type, oldProps, newProps))) {
          if (workInProgress.flags |= 16777216, (renderLanes & 335544128) === renderLanes || maySuspendCommitInSyncRender(type, newProps))
            if (preloadInstance(workInProgress.stateNode, type, newProps))
              workInProgress.flags |= 8192;
            else if (shouldRemainOnPreviousScreen())
              workInProgress.flags |= 8192;
            else
              throw suspendedThenable = noopSuspenseyCommitThenable, SuspenseyCommitException;
        } else
          workInProgress.flags &= -16777217;
      }
      function preloadResourceAndSuspendIfNeeded(workInProgress, resource) {
        if (mayResourceSuspendCommit(resource)) {
          if (workInProgress.flags |= 16777216, !preloadResource(resource))
            if (shouldRemainOnPreviousScreen())
              workInProgress.flags |= 8192;
            else
              throw suspendedThenable = noopSuspenseyCommitThenable, SuspenseyCommitException;
        } else
          workInProgress.flags &= -16777217;
      }
      function scheduleRetryEffect(workInProgress, retryQueue) {
        retryQueue !== null && (workInProgress.flags |= 4);
        workInProgress.flags & 16384 && (retryQueue = workInProgress.tag !== 22 ? claimNextRetryLane() : 536870912, workInProgress.lanes |= retryQueue, workInProgressSuspendedRetryLanes |= retryQueue);
      }
      function cutOffTailIfNeeded(renderState, hasRenderedATailFallback) {
        if (!isHydrating)
          switch (renderState.tailMode) {
            case "visible":
              break;
            case "collapsed":
              for (var tailNode = renderState.tail, lastTailNode = null;tailNode !== null; )
                tailNode.alternate !== null && (lastTailNode = tailNode), tailNode = tailNode.sibling;
              lastTailNode === null ? hasRenderedATailFallback || renderState.tail === null ? renderState.tail = null : renderState.tail.sibling = null : lastTailNode.sibling = null;
              break;
            default:
              hasRenderedATailFallback = renderState.tail;
              for (tailNode = null;hasRenderedATailFallback !== null; )
                hasRenderedATailFallback.alternate !== null && (tailNode = hasRenderedATailFallback), hasRenderedATailFallback = hasRenderedATailFallback.sibling;
              tailNode === null ? renderState.tail = null : tailNode.sibling = null;
          }
      }
      function bubbleProperties(completedWork) {
        var didBailout = completedWork.alternate !== null && completedWork.alternate.child === completedWork.child, newChildLanes = 0, subtreeFlags = 0;
        if (didBailout)
          for (var child$95 = completedWork.child;child$95 !== null; )
            newChildLanes |= child$95.lanes | child$95.childLanes, subtreeFlags |= child$95.subtreeFlags & 1206910976, subtreeFlags |= child$95.flags & 1206910976, child$95.return = completedWork, child$95 = child$95.sibling;
        else
          for (child$95 = completedWork.child;child$95 !== null; )
            newChildLanes |= child$95.lanes | child$95.childLanes, subtreeFlags |= child$95.subtreeFlags, subtreeFlags |= child$95.flags, child$95.return = completedWork, child$95 = child$95.sibling;
        completedWork.subtreeFlags |= subtreeFlags;
        completedWork.childLanes = newChildLanes;
        return didBailout;
      }
      function completeWork(current, workInProgress, renderLanes) {
        var newProps = workInProgress.pendingProps;
        popTreeContext(workInProgress);
        switch (workInProgress.tag) {
          case 16:
          case 15:
          case 0:
          case 11:
          case 7:
          case 8:
          case 12:
          case 9:
          case 14:
            return bubbleProperties(workInProgress), null;
          case 1:
            return bubbleProperties(workInProgress), null;
          case 3:
            renderLanes = workInProgress.stateNode;
            newProps = null;
            current !== null && (newProps = current.memoizedState.cache);
            workInProgress.memoizedState.cache !== newProps && (workInProgress.flags |= 2048);
            popProvider(CacheContext);
            popHostContainer();
            renderLanes.pendingContext && (renderLanes.context = renderLanes.pendingContext, renderLanes.pendingContext = null);
            if (current === null || current.child === null)
              popHydrationState(workInProgress) ? markUpdate(workInProgress) : current === null || current.memoizedState.isDehydrated && (workInProgress.flags & 256) === 0 || (workInProgress.flags |= 1024, upgradeHydrationErrorsToRecoverable());
            updateHostContainer(current, workInProgress);
            bubbleProperties(workInProgress);
            return null;
          case 26:
            if (supportsResources) {
              var { type, memoizedState: nextResource } = workInProgress;
              current === null ? (markUpdate(workInProgress), nextResource !== null ? (bubbleProperties(workInProgress), preloadResourceAndSuspendIfNeeded(workInProgress, nextResource)) : (bubbleProperties(workInProgress), preloadInstanceAndSuspendIfNeeded(workInProgress, type, null, newProps, renderLanes))) : nextResource ? nextResource !== current.memoizedState ? (markUpdate(workInProgress), bubbleProperties(workInProgress), preloadResourceAndSuspendIfNeeded(workInProgress, nextResource)) : (bubbleProperties(workInProgress), workInProgress.flags &= -16777217) : (nextResource = current.memoizedProps, supportsMutation ? nextResource !== newProps && markUpdate(workInProgress) : updateHostComponent(current, workInProgress, type, newProps), bubbleProperties(workInProgress), preloadInstanceAndSuspendIfNeeded(workInProgress, type, nextResource, newProps, renderLanes));
              return null;
            }
          case 27:
            if (supportsSingletons) {
              popHostContext(workInProgress);
              renderLanes = rootInstanceStackCursor.current;
              type = workInProgress.type;
              if (current !== null && workInProgress.stateNode != null)
                supportsMutation ? current.memoizedProps !== newProps && markUpdate(workInProgress) : updateHostComponent(current, workInProgress, type, newProps);
              else {
                if (!newProps) {
                  if (workInProgress.stateNode === null)
                    throw Error(formatProdErrorMessage(166));
                  bubbleProperties(workInProgress);
                  workInProgress.subtreeFlags &= -33554433;
                  return null;
                }
                current = contextStackCursor.current;
                popHydrationState(workInProgress) ? prepareToHydrateHostInstance(workInProgress, current) : (current = resolveSingletonInstance(type, newProps, renderLanes, current, true), workInProgress.stateNode = current, markUpdate(workInProgress));
              }
              bubbleProperties(workInProgress);
              workInProgress.subtreeFlags &= -33554433;
              return null;
            }
          case 5:
            popHostContext(workInProgress);
            type = workInProgress.type;
            if (current !== null && workInProgress.stateNode != null)
              updateHostComponent(current, workInProgress, type, newProps);
            else {
              if (!newProps) {
                if (workInProgress.stateNode === null)
                  throw Error(formatProdErrorMessage(166));
                bubbleProperties(workInProgress);
                workInProgress.subtreeFlags &= -33554433;
                return null;
              }
              nextResource = contextStackCursor.current;
              if (popHydrationState(workInProgress))
                prepareToHydrateHostInstance(workInProgress, nextResource), finalizeHydratedChildren(workInProgress.stateNode, type, newProps, nextResource) && (workInProgress.flags |= 64);
              else {
                var instance$105 = createInstance(type, newProps, rootInstanceStackCursor.current, nextResource, workInProgress);
                markCloned(workInProgress);
                appendAllChildren(instance$105, workInProgress, false, false);
                workInProgress.stateNode = instance$105;
                finalizeInitialChildren(instance$105, type, newProps, nextResource) && markUpdate(workInProgress);
              }
            }
            bubbleProperties(workInProgress);
            workInProgress.subtreeFlags &= -33554433;
            preloadInstanceAndSuspendIfNeeded(workInProgress, workInProgress.type, current === null ? null : current.memoizedProps, workInProgress.pendingProps, renderLanes);
            return null;
          case 6:
            if (current && workInProgress.stateNode != null)
              renderLanes = current.memoizedProps, supportsMutation ? renderLanes !== newProps && markUpdate(workInProgress) : supportsPersistence && (renderLanes !== newProps ? (current = rootInstanceStackCursor.current, renderLanes = contextStackCursor.current, markCloned(workInProgress), workInProgress.stateNode = createTextInstance(newProps, current, renderLanes, workInProgress)) : workInProgress.stateNode = current.stateNode);
            else {
              if (typeof newProps !== "string" && workInProgress.stateNode === null)
                throw Error(formatProdErrorMessage(166));
              current = rootInstanceStackCursor.current;
              renderLanes = contextStackCursor.current;
              if (popHydrationState(workInProgress)) {
                if (!supportsHydration)
                  throw Error(formatProdErrorMessage(176));
                current = workInProgress.stateNode;
                renderLanes = workInProgress.memoizedProps;
                newProps = null;
                type = hydrationParentFiber;
                if (type !== null)
                  switch (type.tag) {
                    case 27:
                    case 5:
                      newProps = type.memoizedProps;
                  }
                hydrateTextInstance(current, renderLanes, workInProgress, newProps) || throwOnHydrationMismatch(workInProgress, true);
              } else
                markCloned(workInProgress), workInProgress.stateNode = createTextInstance(newProps, current, renderLanes, workInProgress);
            }
            bubbleProperties(workInProgress);
            return null;
          case 31:
            renderLanes = workInProgress.memoizedState;
            if (current === null || current.memoizedState !== null) {
              newProps = popHydrationState(workInProgress);
              if (renderLanes !== null) {
                if (current === null) {
                  if (!newProps)
                    throw Error(formatProdErrorMessage(318));
                  if (!supportsHydration)
                    throw Error(formatProdErrorMessage(556));
                  current = workInProgress.memoizedState;
                  current = current !== null ? current.dehydrated : null;
                  if (!current)
                    throw Error(formatProdErrorMessage(557));
                  hydrateActivityInstance(current, workInProgress);
                } else
                  resetHydrationState(), (workInProgress.flags & 128) === 0 && (workInProgress.memoizedState = null), workInProgress.flags |= 4;
                bubbleProperties(workInProgress);
                current = false;
              } else
                renderLanes = upgradeHydrationErrorsToRecoverable(), current !== null && current.memoizedState !== null && (current.memoizedState.hydrationErrors = renderLanes), current = true;
              if (!current) {
                if (workInProgress.flags & 256)
                  return popSuspenseHandler(workInProgress), workInProgress;
                popSuspenseHandler(workInProgress);
                return null;
              }
              if ((workInProgress.flags & 128) !== 0)
                throw Error(formatProdErrorMessage(558));
            }
            bubbleProperties(workInProgress);
            return null;
          case 13:
            newProps = workInProgress.memoizedState;
            if (current === null || current.memoizedState !== null && current.memoizedState.dehydrated !== null) {
              type = popHydrationState(workInProgress);
              if (newProps !== null && newProps.dehydrated !== null) {
                if (current === null) {
                  if (!type)
                    throw Error(formatProdErrorMessage(318));
                  if (!supportsHydration)
                    throw Error(formatProdErrorMessage(344));
                  type = workInProgress.memoizedState;
                  type = type !== null ? type.dehydrated : null;
                  if (!type)
                    throw Error(formatProdErrorMessage(317));
                  hydrateSuspenseInstance(type, workInProgress);
                } else
                  resetHydrationState(), (workInProgress.flags & 128) === 0 && (workInProgress.memoizedState = null), workInProgress.flags |= 4;
                bubbleProperties(workInProgress);
                type = false;
              } else
                type = upgradeHydrationErrorsToRecoverable(), current !== null && current.memoizedState !== null && (current.memoizedState.hydrationErrors = type), type = true;
              if (!type) {
                if (workInProgress.flags & 256)
                  return popSuspenseHandler(workInProgress), workInProgress;
                popSuspenseHandler(workInProgress);
                return null;
              }
            }
            popSuspenseHandler(workInProgress);
            if ((workInProgress.flags & 128) !== 0)
              return workInProgress.lanes = renderLanes, workInProgress;
            renderLanes = newProps !== null;
            current = current !== null && current.memoizedState !== null;
            renderLanes && (newProps = workInProgress.child, type = null, newProps.alternate !== null && newProps.alternate.memoizedState !== null && newProps.alternate.memoizedState.cachePool !== null && (type = newProps.alternate.memoizedState.cachePool.pool), nextResource = null, newProps.memoizedState !== null && newProps.memoizedState.cachePool !== null && (nextResource = newProps.memoizedState.cachePool.pool), nextResource !== type && (newProps.flags |= 2048));
            renderLanes !== current && renderLanes && (workInProgress.child.flags |= 8192);
            scheduleRetryEffect(workInProgress, workInProgress.updateQueue);
            bubbleProperties(workInProgress);
            return null;
          case 4:
            return popHostContainer(), updateHostContainer(current, workInProgress), current === null && preparePortalMount(workInProgress.stateNode.containerInfo), workInProgress.flags |= 67108864, bubbleProperties(workInProgress), null;
          case 10:
            return popProvider(workInProgress.type), bubbleProperties(workInProgress), null;
          case 19:
            popSuspenseListContext(workInProgress);
            newProps = workInProgress.memoizedState;
            if (newProps === null)
              return bubbleProperties(workInProgress), null;
            type = (workInProgress.flags & 128) !== 0;
            nextResource = newProps.rendering;
            if (nextResource === null)
              if (type)
                cutOffTailIfNeeded(newProps, false);
              else {
                if (workInProgressRootExitStatus !== 0 || current !== null && (current.flags & 128) !== 0)
                  for (current = workInProgress.child;current !== null; ) {
                    nextResource = findFirstSuspended(current);
                    if (nextResource !== null) {
                      workInProgress.flags |= 128;
                      cutOffTailIfNeeded(newProps, false);
                      current = nextResource.updateQueue;
                      workInProgress.updateQueue = current;
                      scheduleRetryEffect(workInProgress, current);
                      workInProgress.subtreeFlags = 0;
                      current = renderLanes;
                      for (renderLanes = workInProgress.child;renderLanes !== null; )
                        resetWorkInProgress(renderLanes, current), renderLanes = renderLanes.sibling;
                      pushSuspenseListContext(workInProgress, suspenseStackCursor.current & 1 | 2);
                      isHydrating && pushTreeFork(workInProgress, newProps.treeForkCount);
                      return workInProgress.child;
                    }
                    current = current.sibling;
                  }
                newProps.tail !== null && now() > workInProgressRootRenderTargetTime && (workInProgress.flags |= 128, type = true, cutOffTailIfNeeded(newProps, false), workInProgress.lanes = 4194304);
              }
            else {
              if (!type)
                if (current = findFirstSuspended(nextResource), current !== null) {
                  if (workInProgress.flags |= 128, type = true, current = current.updateQueue, workInProgress.updateQueue = current, scheduleRetryEffect(workInProgress, current), cutOffTailIfNeeded(newProps, true), newProps.tail === null && newProps.tailMode !== "collapsed" && newProps.tailMode !== "visible" && !nextResource.alternate && !isHydrating)
                    return bubbleProperties(workInProgress), null;
                } else
                  2 * now() - newProps.renderingStartTime > workInProgressRootRenderTargetTime && renderLanes !== 536870912 && (workInProgress.flags |= 128, type = true, cutOffTailIfNeeded(newProps, false), workInProgress.lanes = 4194304);
              newProps.isBackwards ? (nextResource.sibling = workInProgress.child, workInProgress.child = nextResource) : (current = newProps.last, current !== null ? current.sibling = nextResource : workInProgress.child = nextResource, newProps.last = nextResource);
            }
            if (newProps.tail !== null) {
              current = newProps.tail;
              a: {
                for (renderLanes = current;renderLanes !== null; ) {
                  if (renderLanes.alternate !== null) {
                    renderLanes = false;
                    break a;
                  }
                  renderLanes = renderLanes.sibling;
                }
                renderLanes = true;
              }
              newProps.rendering = current;
              newProps.tail = current.sibling;
              newProps.renderingStartTime = now();
              current.sibling = null;
              nextResource = suspenseStackCursor.current;
              nextResource = type ? nextResource & 1 | 2 : nextResource & 1;
              newProps.tailMode === "visible" || newProps.tailMode === "collapsed" || !renderLanes || isHydrating ? pushSuspenseListContext(workInProgress, nextResource) : (renderLanes = nextResource, push(suspenseHandlerStackCursor, workInProgress), push(suspenseStackCursor, renderLanes), shellBoundary === null && (shellBoundary = workInProgress));
              isHydrating && pushTreeFork(workInProgress, newProps.treeForkCount);
              return current;
            }
            bubbleProperties(workInProgress);
            return null;
          case 22:
          case 23:
            return popSuspenseHandler(workInProgress), popHiddenContext(), newProps = workInProgress.memoizedState !== null, current !== null ? current.memoizedState !== null !== newProps && (workInProgress.flags |= 8192) : newProps && (workInProgress.flags |= 8192), newProps ? (renderLanes & 536870912) !== 0 && (workInProgress.flags & 128) === 0 && (bubbleProperties(workInProgress), workInProgress.subtreeFlags & 6 && (workInProgress.flags |= 8192)) : bubbleProperties(workInProgress), renderLanes = workInProgress.updateQueue, renderLanes !== null && scheduleRetryEffect(workInProgress, renderLanes.retryQueue), renderLanes = null, current !== null && current.memoizedState !== null && current.memoizedState.cachePool !== null && (renderLanes = current.memoizedState.cachePool.pool), newProps = null, workInProgress.memoizedState !== null && workInProgress.memoizedState.cachePool !== null && (newProps = workInProgress.memoizedState.cachePool.pool), newProps !== renderLanes && (workInProgress.flags |= 2048), current !== null && pop(resumedCache), null;
          case 24:
            return renderLanes = null, current !== null && (renderLanes = current.memoizedState.cache), workInProgress.memoizedState.cache !== renderLanes && (workInProgress.flags |= 2048), popProvider(CacheContext), bubbleProperties(workInProgress), null;
          case 25:
            return null;
          case 30:
            return workInProgress.flags |= 33554432, bubbleProperties(workInProgress), null;
        }
        throw Error(formatProdErrorMessage(156, workInProgress.tag));
      }
      function unwindWork(current, workInProgress) {
        popTreeContext(workInProgress);
        switch (workInProgress.tag) {
          case 1:
            return current = workInProgress.flags, current & 65536 ? (workInProgress.flags = current & -65537 | 128, workInProgress) : null;
          case 3:
            return popProvider(CacheContext), popHostContainer(), current = workInProgress.flags, (current & 65536) !== 0 && (current & 128) === 0 ? (workInProgress.flags = current & -65537 | 128, workInProgress) : null;
          case 26:
          case 27:
          case 5:
            return popHostContext(workInProgress), null;
          case 31:
            if (workInProgress.memoizedState !== null) {
              popSuspenseHandler(workInProgress);
              if (workInProgress.alternate === null)
                throw Error(formatProdErrorMessage(340));
              resetHydrationState();
            }
            current = workInProgress.flags;
            return current & 65536 ? (workInProgress.flags = current & -65537 | 128, workInProgress) : null;
          case 13:
            popSuspenseHandler(workInProgress);
            current = workInProgress.memoizedState;
            if (current !== null && current.dehydrated !== null) {
              if (workInProgress.alternate === null)
                throw Error(formatProdErrorMessage(340));
              resetHydrationState();
            }
            current = workInProgress.flags;
            return current & 65536 ? (workInProgress.flags = current & -65537 | 128, workInProgress) : null;
          case 19:
            return popSuspenseListContext(workInProgress), current = workInProgress.flags, current & 65536 ? (workInProgress.flags = current & -65537 | 128, current = workInProgress.memoizedState, current !== null && (current.rendering = null, current.tail = null), workInProgress.flags |= 4, workInProgress) : null;
          case 4:
            return popHostContainer(), null;
          case 10:
            return popProvider(workInProgress.type), null;
          case 22:
          case 23:
            return popSuspenseHandler(workInProgress), popHiddenContext(), current !== null && pop(resumedCache), current = workInProgress.flags, current & 65536 ? (workInProgress.flags = current & -65537 | 128, workInProgress) : null;
          case 24:
            return popProvider(CacheContext), null;
          case 25:
            return null;
          default:
            return null;
        }
      }
      function unwindInterruptedWork(current, interruptedWork) {
        popTreeContext(interruptedWork);
        switch (interruptedWork.tag) {
          case 3:
            popProvider(CacheContext);
            popHostContainer();
            break;
          case 26:
          case 27:
          case 5:
            popHostContext(interruptedWork);
            break;
          case 4:
            popHostContainer();
            break;
          case 31:
            interruptedWork.memoizedState !== null && popSuspenseHandler(interruptedWork);
            break;
          case 13:
            popSuspenseHandler(interruptedWork);
            break;
          case 19:
            popSuspenseListContext(interruptedWork);
            break;
          case 10:
            popProvider(interruptedWork.type);
            break;
          case 22:
          case 23:
            popSuspenseHandler(interruptedWork);
            popHiddenContext();
            current !== null && pop(resumedCache);
            break;
          case 24:
            popProvider(CacheContext);
        }
      }
      function commitHookEffectListMount(flags, finishedWork) {
        try {
          var updateQueue = finishedWork.updateQueue, lastEffect = updateQueue !== null ? updateQueue.lastEffect : null;
          if (lastEffect !== null) {
            var firstEffect = lastEffect.next;
            updateQueue = firstEffect;
            do {
              if ((updateQueue.tag & flags) === flags) {
                lastEffect = undefined;
                var create = updateQueue.create, inst = updateQueue.inst;
                lastEffect = create();
                inst.destroy = lastEffect;
              }
              updateQueue = updateQueue.next;
            } while (updateQueue !== firstEffect);
          }
        } catch (error) {
          captureCommitPhaseError(finishedWork, finishedWork.return, error);
        }
      }
      function commitHookEffectListUnmount(flags, finishedWork, nearestMountedAncestor$jscomp$0) {
        try {
          var updateQueue = finishedWork.updateQueue, lastEffect = updateQueue !== null ? updateQueue.lastEffect : null;
          if (lastEffect !== null) {
            var firstEffect = lastEffect.next;
            updateQueue = firstEffect;
            do {
              if ((updateQueue.tag & flags) === flags) {
                var inst = updateQueue.inst, destroy = inst.destroy;
                if (destroy !== undefined) {
                  inst.destroy = undefined;
                  lastEffect = finishedWork;
                  var nearestMountedAncestor = nearestMountedAncestor$jscomp$0, destroy_ = destroy;
                  try {
                    destroy_();
                  } catch (error) {
                    captureCommitPhaseError(lastEffect, nearestMountedAncestor, error);
                  }
                }
              }
              updateQueue = updateQueue.next;
            } while (updateQueue !== firstEffect);
          }
        } catch (error) {
          captureCommitPhaseError(finishedWork, finishedWork.return, error);
        }
      }
      function commitClassCallbacks(finishedWork) {
        var updateQueue = finishedWork.updateQueue;
        if (updateQueue !== null) {
          var instance = finishedWork.stateNode;
          try {
            commitCallbacks(updateQueue, instance);
          } catch (error) {
            captureCommitPhaseError(finishedWork, finishedWork.return, error);
          }
        }
      }
      function safelyCallComponentWillUnmount(current, nearestMountedAncestor, instance) {
        instance.props = resolveClassComponentProps(current.type, current.memoizedProps);
        instance.state = current.memoizedState;
        try {
          instance.componentWillUnmount();
        } catch (error) {
          captureCommitPhaseError(current, nearestMountedAncestor, error);
        }
      }
      function safelyAttachRef(current, nearestMountedAncestor) {
        try {
          var ref = current.ref;
          if (ref !== null) {
            switch (current.tag) {
              case 26:
              case 27:
              case 5:
                var instanceToUse = getPublicInstance(current.stateNode);
                break;
              case 30:
                var instance = current.stateNode, name = getViewTransitionName(current.memoizedProps, instance);
                if (instance.ref === null || instance.ref.name !== name)
                  instance.ref = createViewTransitionInstance(name);
                instanceToUse = instance.ref;
                break;
              case 7:
                current.stateNode === null && (current.stateNode = createFragmentInstance(current));
                instanceToUse = current.stateNode;
                break;
              default:
                instanceToUse = current.stateNode;
            }
            typeof ref === "function" ? current.refCleanup = ref(instanceToUse) : ref.current = instanceToUse;
          }
        } catch (error) {
          captureCommitPhaseError(current, nearestMountedAncestor, error);
        }
      }
      function safelyDetachRef(current, nearestMountedAncestor) {
        var ref = current.ref, refCleanup = current.refCleanup;
        if (ref !== null)
          if (typeof refCleanup === "function")
            try {
              refCleanup();
            } catch (error) {
              captureCommitPhaseError(current, nearestMountedAncestor, error);
            } finally {
              current.refCleanup = null, current = current.alternate, current != null && (current.refCleanup = null);
            }
          else if (typeof ref === "function")
            try {
              ref(null);
            } catch (error$130) {
              captureCommitPhaseError(current, nearestMountedAncestor, error$130);
            }
          else
            ref.current = null;
      }
      function pushMutationContext() {
        var prev = viewTransitionMutationContext;
        viewTransitionMutationContext = false;
        return prev;
      }
      function commitNewChildToFragmentInstances(fiber, parentFragmentInstances) {
        if ((fiber.tag === 5 || fiber.tag === 27 || fiber.tag === 6) && fiber.alternate === null && parentFragmentInstances !== null)
          for (var i = 0;i < parentFragmentInstances.length; i++)
            commitNewChildToFragmentInstance(fiber.stateNode, parentFragmentInstances[i]);
      }
      function commitFragmentInstanceInsertionEffects(fiber) {
        for (var parent = fiber.return;parent !== null; ) {
          isFragmentInstanceParent(parent) && commitNewChildToFragmentInstance(fiber.stateNode, parent.stateNode);
          if (isFragmentInstanceHostBoundary(parent))
            break;
          parent = parent.return;
        }
      }
      function commitFragmentInstanceDeletionEffects(fiber) {
        for (var parent = fiber.return;parent !== null; ) {
          isFragmentInstanceParent(parent) && deleteChildFromFragmentInstance(fiber.stateNode, parent.stateNode);
          if (isFragmentInstanceHostBoundary(parent))
            break;
          parent = parent.return;
        }
      }
      function isFragmentInstanceHostBoundary(fiber) {
        return fiber.tag === 5 || fiber.tag === 3 || (supportsSingletons ? fiber.tag === 27 : false);
      }
      function isFragmentInstanceParent(fiber) {
        return fiber && fiber.tag === 7 && fiber.stateNode !== null;
      }
      function commitHostMount(finishedWork) {
        var { type, memoizedProps: props, stateNode: instance } = finishedWork;
        try {
          commitMount(instance, type, props, finishedWork);
        } catch (error) {
          captureCommitPhaseError(finishedWork, finishedWork.return, error);
        }
      }
      function commitHostUpdate(finishedWork, newProps, oldProps) {
        try {
          commitUpdate(finishedWork.stateNode, finishedWork.type, oldProps, newProps, finishedWork);
        } catch (error) {
          captureCommitPhaseError(finishedWork, finishedWork.return, error);
        }
      }
      function isHostParent(fiber) {
        return fiber.tag === 5 || fiber.tag === 3 || (supportsResources ? fiber.tag === 26 : false) || (supportsSingletons ? fiber.tag === 27 && isSingletonScope(fiber.type) : false) || fiber.tag === 4;
      }
      function getHostSibling(fiber) {
        a:
          for (;; ) {
            for (;fiber.sibling === null; ) {
              if (fiber.return === null || isHostParent(fiber.return))
                return null;
              fiber = fiber.return;
            }
            fiber.sibling.return = fiber.return;
            for (fiber = fiber.sibling;fiber.tag !== 5 && fiber.tag !== 6 && fiber.tag !== 18; ) {
              if (supportsSingletons && fiber.tag === 27 && isSingletonScope(fiber.type))
                continue a;
              if (fiber.flags & 2)
                continue a;
              if (fiber.child === null || fiber.tag === 4)
                continue a;
              else
                fiber.child.return = fiber, fiber = fiber.child;
            }
            if (!(fiber.flags & 2))
              return fiber.stateNode;
          }
      }
      function insertOrAppendPlacementNodeIntoContainer(node, before, parent, parentFragmentInstances) {
        var tag = node.tag;
        if (tag === 5 || tag === 6)
          tag = node.stateNode, before ? insertInContainerBefore(parent, tag, before) : appendChildToContainer(parent, tag), commitNewChildToFragmentInstances(node, parentFragmentInstances), viewTransitionMutationContext = true;
        else if (tag !== 4 && (supportsSingletons && tag === 27 && (commitNewChildToFragmentInstances(node, parentFragmentInstances), parentFragmentInstances = null, isSingletonScope(node.type) && (parent = node.stateNode, before = null)), node = node.child, node !== null))
          for (insertOrAppendPlacementNodeIntoContainer(node, before, parent, parentFragmentInstances), node = node.sibling;node !== null; )
            insertOrAppendPlacementNodeIntoContainer(node, before, parent, parentFragmentInstances), node = node.sibling;
      }
      function insertOrAppendPlacementNode(node, before, parent, parentFragmentInstances) {
        var tag = node.tag;
        if (tag === 5 || tag === 6)
          tag = node.stateNode, before ? insertBefore(parent, tag, before) : appendChild(parent, tag), commitNewChildToFragmentInstances(node, parentFragmentInstances), viewTransitionMutationContext = true;
        else if (tag !== 4 && (supportsSingletons && tag === 27 && (commitNewChildToFragmentInstances(node, parentFragmentInstances), parentFragmentInstances = null, isSingletonScope(node.type) && (parent = node.stateNode)), node = node.child, node !== null))
          for (insertOrAppendPlacementNode(node, before, parent, parentFragmentInstances), node = node.sibling;node !== null; )
            insertOrAppendPlacementNode(node, before, parent, parentFragmentInstances), node = node.sibling;
      }
      function commitImmutablePlacementNodeToFragmentInstances(finishedWork, parentFragmentInstances) {
        if (finishedWork.tag === 5 || supportsSingletons && finishedWork.tag === 27)
          commitNewChildToFragmentInstances(finishedWork, parentFragmentInstances);
        else if (finishedWork.tag !== 4 && (finishedWork = finishedWork.child, finishedWork !== null))
          for (commitImmutablePlacementNodeToFragmentInstances(finishedWork, parentFragmentInstances), finishedWork = finishedWork.sibling;finishedWork !== null; )
            commitImmutablePlacementNodeToFragmentInstances(finishedWork, parentFragmentInstances), finishedWork = finishedWork.sibling;
      }
      function commitHostPortalContainerChildren(portal, finishedWork, pendingChildren) {
        portal = portal.containerInfo;
        try {
          replaceContainerChildren(portal, pendingChildren);
        } catch (error) {
          captureCommitPhaseError(finishedWork, finishedWork.return, error);
        }
      }
      function commitHostSingletonAcquisition(finishedWork) {
        var { stateNode: singleton, memoizedProps: props } = finishedWork;
        try {
          acquireSingletonInstance(finishedWork.type, props, singleton, finishedWork);
        } catch (error) {
          captureCommitPhaseError(finishedWork, finishedWork.return, error);
        }
      }
      function trackEnterViewTransitions(placement) {
        if (placement.tag === 30 || (placement.subtreeFlags & 33554432) !== 0)
          shouldStartViewTransition = true;
      }
      function pushViewTransitionCancelableScope() {
        var prevChildren = viewTransitionCancelableChildren;
        viewTransitionCancelableChildren = null;
        return prevChildren;
      }
      function applyViewTransitionToHostInstances(fiber, name, className, collectMeasurements, stopAtNestedViewTransitions) {
        viewTransitionHostInstanceIdx = 0;
        return applyViewTransitionToHostInstancesRecursive(fiber.child, name, className, collectMeasurements, stopAtNestedViewTransitions);
      }
      function applyViewTransitionToHostInstancesRecursive(child, name, className, collectMeasurements, stopAtNestedViewTransitions) {
        if (!supportsMutation)
          return false;
        for (var inViewport = false;child !== null; ) {
          if (child.tag === 5) {
            var instance = child.stateNode;
            if (collectMeasurements !== null) {
              var measurement = measureInstance(instance);
              collectMeasurements.push(measurement);
              wasInstanceInViewport(measurement) && (inViewport = true);
            } else
              inViewport || wasInstanceInViewport(measureInstance(instance)) && (inViewport = true);
            shouldStartViewTransition = true;
            applyViewTransitionName(instance, viewTransitionHostInstanceIdx === 0 ? name : name + "_" + viewTransitionHostInstanceIdx, className);
            viewTransitionHostInstanceIdx++;
          } else if (child.tag !== 22 || child.memoizedState === null)
            child.tag === 30 && stopAtNestedViewTransitions || applyViewTransitionToHostInstancesRecursive(child.child, name, className, collectMeasurements, stopAtNestedViewTransitions) && (inViewport = true);
          child = child.sibling;
        }
        return inViewport;
      }
      function restoreViewTransitionOnHostInstances(child, stopAtNestedViewTransitions) {
        if (supportsMutation)
          for (;child !== null; ) {
            if (child.tag === 5)
              restoreViewTransitionName(child.stateNode, child.memoizedProps);
            else if (child.tag !== 22 || child.memoizedState === null)
              child.tag === 30 && stopAtNestedViewTransitions || restoreViewTransitionOnHostInstances(child.child, stopAtNestedViewTransitions);
            child = child.sibling;
          }
      }
      function commitAppearingPairViewTransitions(placement) {
        if ((placement.subtreeFlags & 18874368) !== 0)
          for (placement = placement.child;placement !== null; ) {
            if (placement.tag !== 22 || placement.memoizedState === null) {
              if (commitAppearingPairViewTransitions(placement), placement.tag === 30 && (placement.flags & 18874368) !== 0 && placement.stateNode.paired) {
                var props = placement.memoizedProps;
                if (props.name == null || props.name === "auto")
                  throw Error(formatProdErrorMessage(544));
                var name = props.name;
                props = getViewTransitionClassName(props.default, props.share);
                props !== "none" && (applyViewTransitionToHostInstances(placement, name, props, null, false) || restoreViewTransitionOnHostInstances(placement.child, false));
              }
            }
            placement = placement.sibling;
          }
      }
      function commitEnterViewTransitions(placement, gesture) {
        if (placement.tag === 30) {
          var state = placement.stateNode, props = placement.memoizedProps, name = getViewTransitionName(props, state), className = getViewTransitionClassName(props.default, state.paired ? props.share : props.enter);
          className !== "none" ? applyViewTransitionToHostInstances(placement, name, className, null, false) ? (commitAppearingPairViewTransitions(placement), state.paired || gesture || scheduleViewTransitionEvent(placement, props.onEnter)) : restoreViewTransitionOnHostInstances(placement.child, false) : commitAppearingPairViewTransitions(placement);
        } else if ((placement.subtreeFlags & 33554432) !== 0)
          for (placement = placement.child;placement !== null; )
            commitEnterViewTransitions(placement, gesture), placement = placement.sibling;
        else
          commitAppearingPairViewTransitions(placement);
      }
      function commitDeletedPairViewTransitions(deletion) {
        if (appearingViewTransitions !== null && appearingViewTransitions.size !== 0) {
          var pairs = appearingViewTransitions;
          if ((deletion.subtreeFlags & 18874368) !== 0)
            for (deletion = deletion.child;deletion !== null; ) {
              if (deletion.tag !== 22 || deletion.memoizedState === null) {
                if (deletion.tag === 30 && (deletion.flags & 18874368) !== 0) {
                  var props = deletion.memoizedProps, name = props.name;
                  if (name != null && name !== "auto") {
                    var pair = pairs.get(name);
                    if (pair !== undefined) {
                      var className = getViewTransitionClassName(props.default, props.share);
                      className !== "none" && (applyViewTransitionToHostInstances(deletion, name, className, null, false) ? (className = deletion.stateNode, pair.paired = className, className.paired = pair, scheduleViewTransitionEvent(deletion, props.onShare)) : restoreViewTransitionOnHostInstances(deletion.child, false));
                      pairs.delete(name);
                      if (pairs.size === 0)
                        break;
                    }
                  }
                }
                commitDeletedPairViewTransitions(deletion);
              }
              deletion = deletion.sibling;
            }
        }
      }
      function commitExitViewTransitions(deletion) {
        if (deletion.tag === 30) {
          var props = deletion.memoizedProps, name = getViewTransitionName(props, deletion.stateNode), pair = appearingViewTransitions !== null ? appearingViewTransitions.get(name) : undefined, className = getViewTransitionClassName(props.default, pair !== undefined ? props.share : props.exit);
          className !== "none" && (applyViewTransitionToHostInstances(deletion, name, className, null, false) ? pair !== undefined ? (className = deletion.stateNode, pair.paired = className, className.paired = pair, appearingViewTransitions.delete(name), scheduleViewTransitionEvent(deletion, props.onShare)) : scheduleViewTransitionEvent(deletion, props.onExit) : restoreViewTransitionOnHostInstances(deletion.child, false));
          appearingViewTransitions !== null && commitDeletedPairViewTransitions(deletion);
        } else if ((deletion.subtreeFlags & 33554432) !== 0)
          for (deletion = deletion.child;deletion !== null; )
            commitExitViewTransitions(deletion), deletion = deletion.sibling;
        else
          appearingViewTransitions !== null && commitDeletedPairViewTransitions(deletion);
      }
      function commitNestedViewTransitions(changedParent) {
        for (changedParent = changedParent.child;changedParent !== null; ) {
          if (changedParent.tag === 30) {
            var props = changedParent.memoizedProps, name = getViewTransitionName(props, changedParent.stateNode);
            props = getViewTransitionClassName(props.default, props.update);
            changedParent.flags &= -5;
            props !== "none" && applyViewTransitionToHostInstances(changedParent, name, props, changedParent.memoizedState = [], false);
          } else
            (changedParent.subtreeFlags & 33554432) !== 0 && commitNestedViewTransitions(changedParent);
          changedParent = changedParent.sibling;
        }
      }
      function restorePairedViewTransitions(parent) {
        if ((parent.subtreeFlags & 18874368) !== 0)
          for (parent = parent.child;parent !== null; ) {
            if (parent.tag !== 22 || parent.memoizedState === null) {
              if (parent.tag === 30 && (parent.flags & 18874368) !== 0) {
                var instance = parent.stateNode;
                instance.paired !== null && (instance.paired = null, restoreViewTransitionOnHostInstances(parent.child, false));
              }
              restorePairedViewTransitions(parent);
            }
            parent = parent.sibling;
          }
      }
      function restoreEnterOrExitViewTransitions(fiber) {
        if (fiber.tag === 30)
          fiber.stateNode.paired = null, restoreViewTransitionOnHostInstances(fiber.child, false), restorePairedViewTransitions(fiber);
        else if ((fiber.subtreeFlags & 33554432) !== 0)
          for (fiber = fiber.child;fiber !== null; )
            restoreEnterOrExitViewTransitions(fiber), fiber = fiber.sibling;
        else
          restorePairedViewTransitions(fiber);
      }
      function restoreNestedViewTransitions(changedParent) {
        for (changedParent = changedParent.child;changedParent !== null; )
          changedParent.tag === 30 ? restoreViewTransitionOnHostInstances(changedParent.child, false) : (changedParent.subtreeFlags & 33554432) !== 0 && restoreNestedViewTransitions(changedParent), changedParent = changedParent.sibling;
      }
      function measureViewTransitionHostInstancesRecursive(parentViewTransition, child, newName, oldName, className, previousMeasurements, stopAtNestedViewTransitions) {
        if (!supportsMutation)
          return false;
        for (var inViewport = false;child !== null; ) {
          if (child.tag === 5) {
            var instance = child.stateNode;
            if (previousMeasurements !== null && viewTransitionHostInstanceIdx < previousMeasurements.length) {
              var previousMeasurement = previousMeasurements[viewTransitionHostInstanceIdx], nextMeasurement = measureInstance(instance);
              if (wasInstanceInViewport(previousMeasurement) || wasInstanceInViewport(nextMeasurement))
                inViewport = true;
              (parentViewTransition.flags & 4) === 0 && hasInstanceChanged(previousMeasurement, nextMeasurement) && (parentViewTransition.flags |= 4);
              hasInstanceAffectedParent(previousMeasurement, nextMeasurement) && (parentViewTransition.flags |= 32);
            } else
              parentViewTransition.flags |= 32;
            (parentViewTransition.flags & 4) !== 0 && applyViewTransitionName(instance, viewTransitionHostInstanceIdx === 0 ? newName : newName + "_" + viewTransitionHostInstanceIdx, className);
            inViewport && (parentViewTransition.flags & 4) !== 0 || (viewTransitionCancelableChildren === null && (viewTransitionCancelableChildren = []), viewTransitionCancelableChildren.push(instance, viewTransitionHostInstanceIdx === 0 ? oldName : oldName + "_" + viewTransitionHostInstanceIdx, child.memoizedProps));
            viewTransitionHostInstanceIdx++;
          } else if (child.tag !== 22 || child.memoizedState === null)
            child.tag === 30 && stopAtNestedViewTransitions ? parentViewTransition.flags |= child.flags & 32 : measureViewTransitionHostInstancesRecursive(parentViewTransition, child.child, newName, oldName, className, previousMeasurements, stopAtNestedViewTransitions) && (inViewport = true);
          child = child.sibling;
        }
        return inViewport;
      }
      function measureNestedViewTransitions(changedParent, gesture) {
        for (changedParent = changedParent.child;changedParent !== null; ) {
          if (changedParent.tag === 30) {
            var props = changedParent.memoizedProps, state = changedParent.stateNode, name = getViewTransitionName(props, state), className = getViewTransitionClassName(props.default, props.update);
            if (gesture) {
              state = state.clones;
              var previousMeasurements = state === null ? null : state.map(measureClonedInstance);
            } else
              previousMeasurements = changedParent.memoizedState, changedParent.memoizedState = null;
            state = changedParent;
            var child = changedParent.child;
            viewTransitionHostInstanceIdx = 0;
            name = measureViewTransitionHostInstancesRecursive(state, child, name, name, className, previousMeasurements, false);
            (changedParent.flags & 4) !== 0 && name && (gesture || scheduleViewTransitionEvent(changedParent, props.onUpdate));
          } else
            (changedParent.subtreeFlags & 33554432) !== 0 && measureNestedViewTransitions(changedParent, gesture);
          changedParent = changedParent.sibling;
        }
      }
      function commitBeforeMutationEffects(root, firstChild, committedLanes) {
        prepareForCommit(root.containerInfo);
        root = (committedLanes & 335544064) === committedLanes;
        nextEffect = firstChild;
        for (firstChild = root ? 9270 : 1024;nextEffect !== null; ) {
          committedLanes = nextEffect;
          if (root) {
            var deletions = committedLanes.deletions;
            if (deletions !== null)
              for (var i = 0;i < deletions.length; i++)
                root && commitExitViewTransitions(deletions[i]);
          }
          if (committedLanes.alternate === null && (committedLanes.flags & 2) !== 0)
            root && trackEnterViewTransitions(committedLanes), commitBeforeMutationEffects_complete(root);
          else {
            if (committedLanes.tag === 22) {
              if (deletions = committedLanes.alternate, committedLanes.memoizedState !== null) {
                deletions !== null && deletions.memoizedState === null && root && commitExitViewTransitions(deletions);
                commitBeforeMutationEffects_complete(root);
                continue;
              } else if (deletions !== null && deletions.memoizedState !== null) {
                root && trackEnterViewTransitions(committedLanes);
                commitBeforeMutationEffects_complete(root);
                continue;
              }
            }
            deletions = committedLanes.child;
            (committedLanes.subtreeFlags & firstChild) !== 0 && deletions !== null ? (deletions.return = committedLanes, nextEffect = deletions) : (root && commitNestedViewTransitions(committedLanes), commitBeforeMutationEffects_complete(root));
          }
        }
        appearingViewTransitions = null;
      }
      function commitBeforeMutationEffects_complete(isViewTransitionEligible$jscomp$0) {
        for (;nextEffect !== null; ) {
          var fiber = nextEffect, isViewTransitionEligible = isViewTransitionEligible$jscomp$0, { alternate: current, flags } = fiber;
          switch (fiber.tag) {
            case 0:
            case 11:
            case 15:
              break;
            case 1:
              if ((flags & 1024) !== 0 && current !== null) {
                isViewTransitionEligible = undefined;
                flags = current.memoizedProps;
                current = current.memoizedState;
                var instance = fiber.stateNode;
                try {
                  var resolvedPrevProps = resolveClassComponentProps(fiber.type, flags);
                  isViewTransitionEligible = instance.getSnapshotBeforeUpdate(resolvedPrevProps, current);
                  instance.__reactInternalSnapshotBeforeUpdate = isViewTransitionEligible;
                } catch (error) {
                  captureCommitPhaseError(fiber, fiber.return, error);
                }
              }
              break;
            case 3:
              (flags & 1024) !== 0 && supportsMutation && clearContainer(fiber.stateNode.containerInfo);
              break;
            case 5:
            case 26:
            case 27:
            case 6:
            case 4:
            case 17:
              break;
            case 30:
              isViewTransitionEligible && current !== null && (isViewTransitionEligible = getViewTransitionName(current.memoizedProps, current.stateNode), flags = fiber.memoizedProps, flags = getViewTransitionClassName(flags.default, flags.update), flags !== "none" && applyViewTransitionToHostInstances(current, isViewTransitionEligible, flags, current.memoizedState = [], true));
              break;
            default:
              if ((flags & 1024) !== 0)
                throw Error(formatProdErrorMessage(163));
          }
          current = fiber.sibling;
          if (current !== null) {
            current.return = fiber.return;
            nextEffect = current;
            break;
          }
          nextEffect = fiber.return;
        }
      }
      function commitLayoutEffectOnFiber(finishedRoot, current, finishedWork) {
        var flags = finishedWork.flags;
        switch (finishedWork.tag) {
          case 0:
          case 11:
          case 15:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            flags & 4 && commitHookEffectListMount(5, finishedWork);
            break;
          case 1:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            if (flags & 4)
              if (finishedRoot = finishedWork.stateNode, current === null)
                try {
                  finishedRoot.componentDidMount();
                } catch (error) {
                  captureCommitPhaseError(finishedWork, finishedWork.return, error);
                }
              else {
                var prevProps = resolveClassComponentProps(finishedWork.type, current.memoizedProps);
                current = current.memoizedState;
                try {
                  finishedRoot.componentDidUpdate(prevProps, current, finishedRoot.__reactInternalSnapshotBeforeUpdate);
                } catch (error$128) {
                  captureCommitPhaseError(finishedWork, finishedWork.return, error$128);
                }
              }
            flags & 64 && commitClassCallbacks(finishedWork);
            flags & 512 && safelyAttachRef(finishedWork, finishedWork.return);
            break;
          case 3:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            if (flags & 64 && (flags = finishedWork.updateQueue, flags !== null)) {
              finishedRoot = null;
              if (finishedWork.child !== null)
                switch (finishedWork.child.tag) {
                  case 27:
                  case 5:
                    finishedRoot = getPublicInstance(finishedWork.child.stateNode);
                    break;
                  case 1:
                    finishedRoot = finishedWork.child.stateNode;
                }
              try {
                commitCallbacks(flags, finishedRoot);
              } catch (error) {
                captureCommitPhaseError(finishedWork, finishedWork.return, error);
              }
            }
            break;
          case 27:
            supportsSingletons && current === null && flags & 4 && commitHostSingletonAcquisition(finishedWork);
          case 26:
          case 5:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            if (current === null) {
              if (flags & 4)
                commitHostMount(finishedWork);
              else if (flags & 64) {
                finishedRoot = finishedWork.type;
                current = finishedWork.memoizedProps;
                prevProps = finishedWork.stateNode;
                try {
                  commitHydratedInstance(prevProps, finishedRoot, current, finishedWork);
                } catch (error) {
                  captureCommitPhaseError(finishedWork, finishedWork.return, error);
                }
              }
            }
            flags & 512 && safelyAttachRef(finishedWork, finishedWork.return);
            break;
          case 12:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            break;
          case 31:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            flags & 4 && commitActivityHydrationCallbacks(finishedRoot, finishedWork);
            break;
          case 13:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            flags & 4 && commitSuspenseHydrationCallbacks(finishedRoot, finishedWork);
            flags & 64 && (flags = finishedWork.memoizedState, flags !== null && (flags = flags.dehydrated, flags !== null && (finishedWork = retryDehydratedSuspenseBoundary.bind(null, finishedWork), registerSuspenseInstanceRetry(flags, finishedWork))));
            break;
          case 22:
            flags = finishedWork.memoizedState !== null || offscreenSubtreeIsHidden;
            if (!flags) {
              var newOffscreenSubtreeWasHidden = current !== null && current.memoizedState !== null || offscreenSubtreeWasHidden;
              current = offscreenSubtreeIsHidden;
              prevProps = offscreenSubtreeWasHidden;
              offscreenSubtreeIsHidden = flags;
              (offscreenSubtreeWasHidden = newOffscreenSubtreeWasHidden) && !prevProps ? (flags = supportsSingletons ? 2 : 0, (finishedWork.subtreeFlags & 8772) !== 0 && (flags |= 1), recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, flags)) : recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
              offscreenSubtreeIsHidden = current;
              offscreenSubtreeWasHidden = prevProps;
            }
            break;
          case 30:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
            flags & 512 && safelyAttachRef(finishedWork, finishedWork.return);
            break;
          case 7:
            flags & 512 && safelyAttachRef(finishedWork, finishedWork.return);
          default:
            recursivelyTraverseLayoutEffects(finishedRoot, finishedWork);
        }
      }
      function hideOrUnhideAllChildren(parentFiber, isHidden) {
        if (supportsMutation)
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            hideOrUnhideAllChildrenOnFiber(parentFiber, isHidden), parentFiber = parentFiber.sibling;
      }
      function hideOrUnhideAllChildrenOnFiber(fiber, isHidden) {
        if (supportsMutation)
          switch (fiber.tag) {
            case 5:
            case 26:
              try {
                var instance = fiber.stateNode;
                isHidden ? hideInstance(instance) : unhideInstance(fiber.stateNode, fiber.memoizedProps);
              } catch (error) {
                captureCommitPhaseError(fiber, fiber.return, error);
              }
              hideOrUnhideNearestPortals(fiber, isHidden);
              break;
            case 6:
              try {
                var instance$jscomp$0 = fiber.stateNode;
                isHidden ? hideTextInstance(instance$jscomp$0) : unhideTextInstance(instance$jscomp$0, fiber.memoizedProps);
                viewTransitionMutationContext = true;
              } catch (error) {
                captureCommitPhaseError(fiber, fiber.return, error);
              }
              break;
            case 18:
              try {
                var instance$jscomp$1 = fiber.stateNode;
                isHidden ? hideDehydratedBoundary(instance$jscomp$1) : unhideDehydratedBoundary(fiber.stateNode);
              } catch (error) {
                captureCommitPhaseError(fiber, fiber.return, error);
              }
              break;
            case 22:
            case 23:
              fiber.memoizedState === null && hideOrUnhideAllChildren(fiber, isHidden);
              break;
            default:
              hideOrUnhideAllChildren(fiber, isHidden);
          }
      }
      function hideOrUnhideNearestPortals(parentFiber, isHidden$jscomp$0) {
        if (supportsMutation && parentFiber.subtreeFlags & 67108864)
          for (parentFiber = parentFiber.child;parentFiber !== null; ) {
            var fiber = parentFiber, isHidden = isHidden$jscomp$0;
            if (supportsMutation)
              switch (fiber.tag) {
                case 4:
                  hideOrUnhideAllChildrenOnFiber(fiber, isHidden);
                  break;
                case 22:
                  fiber.memoizedState === null && hideOrUnhideNearestPortals(fiber, isHidden);
                  break;
                default:
                  hideOrUnhideNearestPortals(fiber, isHidden);
              }
            parentFiber = parentFiber.sibling;
          }
      }
      function detachFiberAfterEffects(fiber) {
        var alternate = fiber.alternate;
        alternate !== null && (fiber.alternate = null, detachFiberAfterEffects(alternate));
        fiber.child = null;
        fiber.deletions = null;
        fiber.sibling = null;
        fiber.tag === 5 && (alternate = fiber.stateNode, alternate !== null && detachDeletedInstance(alternate));
        fiber.stateNode = null;
        fiber.return = null;
        fiber.dependencies = null;
        fiber.memoizedProps = null;
        fiber.memoizedState = null;
        fiber.pendingProps = null;
        fiber.stateNode = null;
        fiber.updateQueue = null;
      }
      function recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, parent) {
        for (parent = parent.child;parent !== null; )
          commitDeletionEffectsOnFiber(finishedRoot, nearestMountedAncestor, parent), parent = parent.sibling;
      }
      function commitDeletionEffectsOnFiber(finishedRoot, nearestMountedAncestor, deletedFiber) {
        if (injectedHook && typeof injectedHook.onCommitFiberUnmount === "function")
          try {
            injectedHook.onCommitFiberUnmount(rendererID, deletedFiber);
          } catch (err) {}
        switch (deletedFiber.tag) {
          case 26:
            if (supportsResources) {
              offscreenSubtreeWasHidden || safelyDetachRef(deletedFiber, nearestMountedAncestor);
              recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
              deletedFiber.memoizedState ? releaseResource(deletedFiber.memoizedState) : deletedFiber.stateNode && (offscreenSubtreeWasHidden || unmountHoistable(deletedFiber.stateNode));
              break;
            }
          case 27:
            if (supportsSingletons) {
              offscreenSubtreeWasHidden || safelyDetachRef(deletedFiber, nearestMountedAncestor);
              commitFragmentInstanceDeletionEffects(deletedFiber);
              var prevHostParent = hostParent, prevHostParentIsContainer = hostParentIsContainer;
              isSingletonScope(deletedFiber.type) && (hostParent = deletedFiber.stateNode, hostParentIsContainer = false);
              recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
              releaseSingletonInstance(deletedFiber.stateNode, deletedFiber.type, deletedFiber.memoizedProps);
              hostParent = prevHostParent;
              hostParentIsContainer = prevHostParentIsContainer;
              break;
            }
          case 5:
            offscreenSubtreeWasHidden || safelyDetachRef(deletedFiber, nearestMountedAncestor), commitFragmentInstanceDeletionEffects(deletedFiber);
          case 6:
            deletedFiber.tag === 6 && commitFragmentInstanceDeletionEffects(deletedFiber);
            if (supportsMutation) {
              if (prevHostParent = hostParent, prevHostParentIsContainer = hostParentIsContainer, hostParent = null, recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber), hostParent = prevHostParent, hostParentIsContainer = prevHostParentIsContainer, hostParent !== null)
                if (hostParentIsContainer)
                  try {
                    removeChildFromContainer(hostParent, deletedFiber.stateNode), viewTransitionMutationContext = true;
                  } catch (error) {
                    captureCommitPhaseError(deletedFiber, nearestMountedAncestor, error);
                  }
                else
                  try {
                    removeChild(hostParent, deletedFiber.stateNode), viewTransitionMutationContext = true;
                  } catch (error) {
                    captureCommitPhaseError(deletedFiber, nearestMountedAncestor, error);
                  }
            } else
              recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            break;
          case 18:
            supportsMutation && hostParent !== null && (hostParentIsContainer ? clearSuspenseBoundaryFromContainer(hostParent, deletedFiber.stateNode) : clearSuspenseBoundary(hostParent, deletedFiber.stateNode));
            break;
          case 4:
            supportsMutation ? (prevHostParent = hostParent, prevHostParentIsContainer = hostParentIsContainer, hostParent = deletedFiber.stateNode.containerInfo, hostParentIsContainer = true, recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber), hostParent = prevHostParent, hostParentIsContainer = prevHostParentIsContainer) : (supportsPersistence && commitHostPortalContainerChildren(deletedFiber.stateNode, deletedFiber, createContainerChildSet()), recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber));
            break;
          case 0:
          case 11:
          case 14:
          case 15:
            commitHookEffectListUnmount(2, deletedFiber, nearestMountedAncestor);
            offscreenSubtreeWasHidden || commitHookEffectListUnmount(4, deletedFiber, nearestMountedAncestor);
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            break;
          case 1:
            offscreenSubtreeWasHidden || (safelyDetachRef(deletedFiber, nearestMountedAncestor), prevHostParent = deletedFiber.stateNode, typeof prevHostParent.componentWillUnmount === "function" && safelyCallComponentWillUnmount(deletedFiber, nearestMountedAncestor, prevHostParent));
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            break;
          case 21:
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            break;
          case 22:
            offscreenSubtreeWasHidden = (prevHostParent = offscreenSubtreeWasHidden) || deletedFiber.memoizedState !== null;
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            offscreenSubtreeWasHidden = prevHostParent;
            break;
          case 30:
            safelyDetachRef(deletedFiber, nearestMountedAncestor);
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            break;
          case 7:
            offscreenSubtreeWasHidden || safelyDetachRef(deletedFiber, nearestMountedAncestor);
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
            break;
          default:
            recursivelyTraverseDeletionEffects(finishedRoot, nearestMountedAncestor, deletedFiber);
        }
      }
      function commitActivityHydrationCallbacks(finishedRoot, finishedWork) {
        if (supportsHydration && finishedWork.memoizedState === null && (finishedRoot = finishedWork.alternate, finishedRoot !== null && (finishedRoot = finishedRoot.memoizedState, finishedRoot !== null))) {
          finishedRoot = finishedRoot.dehydrated;
          try {
            commitHydratedActivityInstance(finishedRoot);
          } catch (error) {
            captureCommitPhaseError(finishedWork, finishedWork.return, error);
          }
        }
      }
      function commitSuspenseHydrationCallbacks(finishedRoot, finishedWork) {
        if (supportsHydration && finishedWork.memoizedState === null && (finishedRoot = finishedWork.alternate, finishedRoot !== null && (finishedRoot = finishedRoot.memoizedState, finishedRoot !== null && (finishedRoot = finishedRoot.dehydrated, finishedRoot !== null))))
          try {
            commitHydratedSuspenseInstance(finishedRoot);
          } catch (error) {
            captureCommitPhaseError(finishedWork, finishedWork.return, error);
          }
      }
      function getRetryCache(finishedWork) {
        switch (finishedWork.tag) {
          case 31:
          case 13:
          case 19:
            var retryCache = finishedWork.stateNode;
            retryCache === null && (retryCache = finishedWork.stateNode = new PossiblyWeakSet);
            return retryCache;
          case 22:
            return finishedWork = finishedWork.stateNode, retryCache = finishedWork._retryCache, retryCache === null && (retryCache = finishedWork._retryCache = new PossiblyWeakSet), retryCache;
          default:
            throw Error(formatProdErrorMessage(435, finishedWork.tag));
        }
      }
      function attachSuspenseRetryListeners(finishedWork, wakeables) {
        var retryCache = getRetryCache(finishedWork);
        wakeables.forEach(function(wakeable) {
          if (!retryCache.has(wakeable)) {
            retryCache.add(wakeable);
            var retry = resolveRetryWakeable.bind(null, finishedWork, wakeable);
            wakeable.then(retry, retry);
          }
        });
      }
      function recursivelyTraverseMutationEffects(root$jscomp$0, parentFiber, lanes) {
        var deletions = parentFiber.deletions;
        if (deletions !== null)
          for (var i = 0;i < deletions.length; i++) {
            var childToDelete = deletions[i], root = root$jscomp$0, returnFiber = parentFiber;
            if (supportsMutation) {
              var parent = returnFiber;
              a:
                for (;parent !== null; ) {
                  switch (parent.tag) {
                    case 27:
                      if (supportsSingletons) {
                        if (isSingletonScope(parent.type)) {
                          hostParent = parent.stateNode;
                          hostParentIsContainer = false;
                          break a;
                        }
                        break;
                      }
                    case 5:
                      hostParent = parent.stateNode;
                      hostParentIsContainer = false;
                      break a;
                    case 3:
                    case 4:
                      hostParent = parent.stateNode.containerInfo;
                      hostParentIsContainer = true;
                      break a;
                  }
                  parent = parent.return;
                }
              if (hostParent === null)
                throw Error(formatProdErrorMessage(160));
              commitDeletionEffectsOnFiber(root, returnFiber, childToDelete);
              hostParent = null;
              hostParentIsContainer = false;
            } else
              commitDeletionEffectsOnFiber(root, returnFiber, childToDelete);
            root = childToDelete.alternate;
            root !== null && (root.return = null);
            childToDelete.return = null;
          }
        if (parentFiber.subtreeFlags & 13886)
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            commitMutationEffectsOnFiber(parentFiber, root$jscomp$0, lanes), parentFiber = parentFiber.sibling;
      }
      function commitMutationEffectsOnFiber(finishedWork, root, lanes) {
        var current = finishedWork.alternate, flags = finishedWork.flags;
        switch (finishedWork.tag) {
          case 0:
          case 11:
          case 14:
          case 15:
            if (flags & 4 && (current = finishedWork.updateQueue, current = current !== null ? current.events : null, current !== null))
              for (var ii = 0;ii < current.length; ii++) {
                var _eventPayloads$ii2 = current[ii];
                _eventPayloads$ii2.ref.impl = _eventPayloads$ii2.nextImpl;
              }
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            flags & 4 && (commitHookEffectListUnmount(3, finishedWork, finishedWork.return), commitHookEffectListMount(3, finishedWork), commitHookEffectListUnmount(5, finishedWork, finishedWork.return));
            break;
          case 1:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            flags & 512 && (offscreenSubtreeWasHidden || current === null || safelyDetachRef(current, current.return));
            flags & 64 && offscreenSubtreeIsHidden && (finishedWork = finishedWork.updateQueue, finishedWork !== null && (root = finishedWork.callbacks, root !== null && (flags = finishedWork.shared.hiddenCallbacks, finishedWork.shared.hiddenCallbacks = flags === null ? root : flags.concat(root))));
            break;
          case 26:
            if (supportsResources) {
              ii = currentHoistableRoot;
              recursivelyTraverseMutationEffects(root, finishedWork, lanes);
              commitReconciliationEffects(finishedWork);
              flags & 512 && (offscreenSubtreeWasHidden || current === null || safelyDetachRef(current, current.return));
              flags & 4 && (lanes = current !== null ? current.memoizedState : null, flags = finishedWork.memoizedState, current === null ? flags === null ? finishedWork.stateNode === null ? finishedWork.stateNode = offscreenSubtreeIsHidden ? createHoistableInstance(finishedWork.type, finishedWork.memoizedProps, root.containerInfo, finishedWork) : hydrateHoistable(ii, finishedWork.type, finishedWork.memoizedProps, finishedWork) : offscreenSubtreeIsHidden || mountHoistable(ii, finishedWork.type, finishedWork.stateNode) : finishedWork.stateNode = acquireResource(ii, flags, finishedWork.memoizedProps) : lanes !== flags ? (lanes === null ? (root = current.stateNode, root === null || offscreenSubtreeWasHidden || unmountHoistable(root)) : releaseResource(lanes), flags === null ? offscreenSubtreeIsHidden || mountHoistable(ii, finishedWork.type, finishedWork.stateNode) : acquireResource(ii, flags, finishedWork.memoizedProps)) : flags === null && finishedWork.stateNode !== null && commitHostUpdate(finishedWork, finishedWork.memoizedProps, current.memoizedProps));
              break;
            }
          case 27:
            if (supportsSingletons) {
              recursivelyTraverseMutationEffects(root, finishedWork, lanes);
              commitReconciliationEffects(finishedWork);
              flags & 512 && (offscreenSubtreeWasHidden || current === null || safelyDetachRef(current, current.return));
              current !== null && flags & 4 && commitHostUpdate(finishedWork, finishedWork.memoizedProps, current.memoizedProps);
              break;
            }
          case 5:
            ii = offscreenDirectParentIsHidden;
            offscreenDirectParentIsHidden = false;
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            offscreenDirectParentIsHidden = ii;
            commitReconciliationEffects(finishedWork);
            flags & 512 && (offscreenSubtreeWasHidden || current === null || safelyDetachRef(current, current.return));
            if (supportsMutation) {
              if (finishedWork.flags & 32) {
                root = finishedWork.stateNode;
                try {
                  resetTextContent(root), viewTransitionMutationContext = true;
                } catch (error) {
                  captureCommitPhaseError(finishedWork, finishedWork.return, error);
                }
              }
              flags & 4 && finishedWork.stateNode != null && (root = finishedWork.memoizedProps, commitHostUpdate(finishedWork, root, current !== null ? current.memoizedProps : root));
              flags & 1024 && (needsFormReset = true);
            } else
              supportsPersistence && finishedWork.alternate !== null && (finishedWork.alternate.stateNode = finishedWork.stateNode);
            break;
          case 6:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            if (flags & 4 && supportsMutation) {
              if (finishedWork.stateNode === null)
                throw Error(formatProdErrorMessage(162));
              root = finishedWork.memoizedProps;
              flags = current !== null ? current.memoizedProps : root;
              lanes = finishedWork.stateNode;
              try {
                commitTextUpdate(lanes, flags, root), viewTransitionMutationContext = true;
              } catch (error) {
                captureCommitPhaseError(finishedWork, finishedWork.return, error);
              }
            }
            break;
          case 3:
            viewTransitionMutationContext = false;
            supportsResources ? (prepareToCommitHoistables(), ii = currentHoistableRoot, currentHoistableRoot = getHoistableRoot(root.containerInfo), recursivelyTraverseMutationEffects(root, finishedWork, lanes), currentHoistableRoot = ii) : recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            if (flags & 4) {
              if (supportsMutation && supportsHydration && current !== null && current.memoizedState.isDehydrated)
                try {
                  commitHydratedContainer(root.containerInfo);
                } catch (error) {
                  captureCommitPhaseError(finishedWork, finishedWork.return, error);
                }
              if (supportsPersistence) {
                flags = root.containerInfo;
                root = root.pendingChildren;
                try {
                  replaceContainerChildren(flags, root), viewTransitionMutationContext = true;
                } catch (error) {
                  captureCommitPhaseError(finishedWork, finishedWork.return, error);
                }
              }
            }
            needsFormReset && (needsFormReset = false, recursivelyResetForms(finishedWork));
            viewTransitionMutationContext = false;
            break;
          case 4:
            current = offscreenDirectParentIsHidden;
            offscreenDirectParentIsHidden = offscreenSubtreeIsHidden;
            ii = pushMutationContext();
            supportsResources ? (_eventPayloads$ii2 = currentHoistableRoot, currentHoistableRoot = getHoistableRoot(finishedWork.stateNode.containerInfo), recursivelyTraverseMutationEffects(root, finishedWork, lanes), commitReconciliationEffects(finishedWork), currentHoistableRoot = _eventPayloads$ii2) : (recursivelyTraverseMutationEffects(root, finishedWork, lanes), commitReconciliationEffects(finishedWork));
            viewTransitionMutationContext && inUpdateViewTransition && (rootViewTransitionAffected = true);
            viewTransitionMutationContext = ii;
            offscreenDirectParentIsHidden = current;
            flags & 4 && supportsPersistence && commitHostPortalContainerChildren(finishedWork.stateNode, finishedWork, finishedWork.stateNode.pendingChildren);
            break;
          case 12:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            break;
          case 31:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            flags & 4 && (root = finishedWork.updateQueue, root !== null && (finishedWork.updateQueue = null, attachSuspenseRetryListeners(finishedWork, root)));
            break;
          case 13:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            finishedWork.child.flags & 8192 && finishedWork.memoizedState !== null !== (current !== null && current.memoizedState !== null) && (globalMostRecentFallbackTime = now());
            flags & 4 && (root = finishedWork.updateQueue, root !== null && (finishedWork.updateQueue = null, attachSuspenseRetryListeners(finishedWork, root)));
            break;
          case 22:
            ii = finishedWork.memoizedState !== null;
            _eventPayloads$ii2 = current !== null && current.memoizedState !== null;
            var prevOffscreenSubtreeIsHidden = offscreenSubtreeIsHidden, prevOffscreenSubtreeWasHidden = offscreenSubtreeWasHidden, prevOffscreenDirectParentIsHidden$147 = offscreenDirectParentIsHidden;
            offscreenSubtreeIsHidden = prevOffscreenSubtreeIsHidden || ii;
            offscreenDirectParentIsHidden = prevOffscreenDirectParentIsHidden$147 || ii;
            offscreenSubtreeWasHidden = prevOffscreenSubtreeWasHidden || _eventPayloads$ii2;
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            offscreenSubtreeWasHidden = prevOffscreenSubtreeWasHidden;
            offscreenDirectParentIsHidden = prevOffscreenDirectParentIsHidden$147;
            offscreenSubtreeIsHidden = prevOffscreenSubtreeIsHidden;
            commitReconciliationEffects(finishedWork);
            flags & 8192 && (root = finishedWork.stateNode, root._visibility = ii ? root._visibility & -2 : root._visibility | 1, !ii || current === null || _eventPayloads$ii2 || offscreenSubtreeIsHidden || offscreenSubtreeWasHidden || (root = supportsSingletons ? 2 : 0, lanes = _eventPayloads$ii2 || offscreenSubtreeWasHidden, current = offscreenSubtreeIsHidden, _eventPayloads$ii2 = offscreenSubtreeWasHidden, offscreenSubtreeIsHidden = ii || offscreenSubtreeIsHidden, offscreenSubtreeWasHidden = lanes, recursivelyTraverseDisappearLayoutEffects(finishedWork, root), offscreenSubtreeIsHidden = current, offscreenSubtreeWasHidden = _eventPayloads$ii2), supportsMutation && (ii || !offscreenDirectParentIsHidden) && hideOrUnhideAllChildren(finishedWork, ii));
            flags & 4 && (root = finishedWork.updateQueue, root !== null && (flags = root.retryQueue, flags !== null && (root.retryQueue = null, attachSuspenseRetryListeners(finishedWork, flags))));
            break;
          case 19:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            flags & 4 && (root = finishedWork.updateQueue, root !== null && (finishedWork.updateQueue = null, attachSuspenseRetryListeners(finishedWork, root)));
            break;
          case 30:
            flags & 512 && (offscreenSubtreeWasHidden || current === null || safelyDetachRef(current, current.return));
            flags = pushMutationContext();
            ii = inUpdateViewTransition;
            _eventPayloads$ii2 = (lanes & 335544064) === lanes;
            prevOffscreenSubtreeIsHidden = finishedWork.memoizedProps;
            inUpdateViewTransition = _eventPayloads$ii2 && getViewTransitionClassName(prevOffscreenSubtreeIsHidden.default, prevOffscreenSubtreeIsHidden.update) !== "none";
            recursivelyTraverseMutationEffects(root, finishedWork, lanes);
            commitReconciliationEffects(finishedWork);
            _eventPayloads$ii2 && current !== null && viewTransitionMutationContext && (finishedWork.flags |= 4);
            inUpdateViewTransition = ii;
            viewTransitionMutationContext = flags;
            break;
          case 21:
            break;
          case 7:
            flags & 512 && (offscreenSubtreeWasHidden || current === null || safelyDetachRef(current, current.return)), current && current.stateNode !== null && updateFragmentInstanceFiber(finishedWork, current.stateNode);
          default:
            recursivelyTraverseMutationEffects(root, finishedWork, lanes), commitReconciliationEffects(finishedWork);
        }
      }
      function commitReconciliationEffects(finishedWork) {
        var flags = finishedWork.flags;
        if (flags & 2) {
          try {
            for (var hostParentFiber, parentFiber = finishedWork.return;parentFiber !== null; ) {
              if (isHostParent(parentFiber)) {
                hostParentFiber = parentFiber;
                break;
              }
              parentFiber = parentFiber.return;
            }
            parentFiber = null;
            for (var parent = finishedWork.return;parent !== null; ) {
              if (isFragmentInstanceParent(parent)) {
                var fragmentInstance = parent.stateNode;
                parentFiber === null ? parentFiber = [fragmentInstance] : parentFiber.push(fragmentInstance);
              }
              if (isFragmentInstanceHostBoundary(parent))
                break;
              parent = parent.return;
            }
            var JSCompiler_inline_result = parentFiber;
            if (supportsMutation) {
              if (hostParentFiber == null)
                throw Error(formatProdErrorMessage(160));
              switch (hostParentFiber.tag) {
                case 27:
                  if (supportsSingletons) {
                    var parent$jscomp$0 = hostParentFiber.stateNode, before = getHostSibling(finishedWork);
                    insertOrAppendPlacementNode(finishedWork, before, parent$jscomp$0, JSCompiler_inline_result);
                    break;
                  }
                case 5:
                  var parent$131 = hostParentFiber.stateNode;
                  hostParentFiber.flags & 32 && (resetTextContent(parent$131), hostParentFiber.flags &= -33);
                  var before$132 = getHostSibling(finishedWork);
                  insertOrAppendPlacementNode(finishedWork, before$132, parent$131, JSCompiler_inline_result);
                  break;
                case 3:
                case 4:
                  var parent$133 = hostParentFiber.stateNode.containerInfo, before$134 = getHostSibling(finishedWork);
                  insertOrAppendPlacementNodeIntoContainer(finishedWork, before$134, parent$133, JSCompiler_inline_result);
                  break;
                default:
                  throw Error(formatProdErrorMessage(161));
              }
            } else
              commitImmutablePlacementNodeToFragmentInstances(finishedWork, JSCompiler_inline_result);
          } catch (error) {
            captureCommitPhaseError(finishedWork, finishedWork.return, error);
          }
          finishedWork.flags &= -3;
        }
        flags & 4096 && (finishedWork.flags &= -4097);
      }
      function recursivelyResetForms(parentFiber) {
        if (parentFiber.subtreeFlags & 1024)
          for (parentFiber = parentFiber.child;parentFiber !== null; ) {
            var fiber = parentFiber;
            recursivelyResetForms(fiber);
            fiber.tag === 5 && fiber.flags & 1024 && resetFormInstance(fiber.stateNode);
            parentFiber = parentFiber.sibling;
          }
      }
      function recursivelyTraverseAfterMutationEffects(root, parentFiber) {
        if (parentFiber.subtreeFlags & 9270)
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            commitAfterMutationEffectsOnFiber(parentFiber, root), parentFiber = parentFiber.sibling;
        else
          measureNestedViewTransitions(parentFiber, false);
      }
      function commitAfterMutationEffectsOnFiber(finishedWork, root) {
        var current = finishedWork.alternate;
        if (current === null)
          commitEnterViewTransitions(finishedWork, false);
        else
          switch (finishedWork.tag) {
            case 3:
              rootViewTransitionNameCanceled = viewTransitionContextChanged = false;
              pushViewTransitionCancelableScope();
              recursivelyTraverseAfterMutationEffects(root, finishedWork);
              if (!viewTransitionContextChanged && !rootViewTransitionAffected) {
                finishedWork = viewTransitionCancelableChildren;
                if (finishedWork !== null)
                  for (var i = 0;i < finishedWork.length; i += 3)
                    cancelViewTransitionName(finishedWork[i], finishedWork[i + 1], finishedWork[i + 2]);
                cancelRootViewTransitionName(root.containerInfo);
                rootViewTransitionNameCanceled = true;
              }
              viewTransitionCancelableChildren = null;
              break;
            case 5:
              recursivelyTraverseAfterMutationEffects(root, finishedWork);
              break;
            case 4:
              i = viewTransitionContextChanged;
              viewTransitionContextChanged = false;
              recursivelyTraverseAfterMutationEffects(root, finishedWork);
              viewTransitionContextChanged && (rootViewTransitionAffected = true);
              viewTransitionContextChanged = i;
              break;
            case 22:
              finishedWork.memoizedState === null && (current.memoizedState !== null ? commitEnterViewTransitions(finishedWork, false) : recursivelyTraverseAfterMutationEffects(root, finishedWork));
              break;
            case 30:
              i = viewTransitionContextChanged;
              var prevCancelableChildren = pushViewTransitionCancelableScope();
              viewTransitionContextChanged = false;
              recursivelyTraverseAfterMutationEffects(root, finishedWork);
              viewTransitionContextChanged && (finishedWork.flags |= 4);
              var props = finishedWork.memoizedProps, state = finishedWork.stateNode;
              root = getViewTransitionName(props, state);
              state = getViewTransitionName(current.memoizedProps, state);
              var className = getViewTransitionClassName(props.default, props.update);
              className === "none" ? current = false : (props = current.memoizedState, current.memoizedState = null, current = finishedWork.child, viewTransitionHostInstanceIdx = 0, current = measureViewTransitionHostInstancesRecursive(finishedWork, current, root, state, className, props, true), viewTransitionHostInstanceIdx !== (props === null ? 0 : props.length) && (finishedWork.flags |= 32));
              (finishedWork.flags & 4) !== 0 && current ? (scheduleViewTransitionEvent(finishedWork, finishedWork.memoizedProps.onUpdate), viewTransitionCancelableChildren = prevCancelableChildren) : prevCancelableChildren !== null && (prevCancelableChildren.push.apply(prevCancelableChildren, viewTransitionCancelableChildren), viewTransitionCancelableChildren = prevCancelableChildren);
              viewTransitionContextChanged = (finishedWork.flags & 32) !== 0 ? true : i;
              break;
            default:
              recursivelyTraverseAfterMutationEffects(root, finishedWork);
          }
      }
      function recursivelyTraverseLayoutEffects(root, parentFiber) {
        if (parentFiber.subtreeFlags & 8772)
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            commitLayoutEffectOnFiber(root, parentFiber.alternate, parentFiber), parentFiber = parentFiber.sibling;
      }
      function recursivelyTraverseDisappearLayoutEffects(parentFiber, layoutEffectTraversalFlags$jscomp$0) {
        for (parentFiber = parentFiber.child;parentFiber !== null; ) {
          var finishedWork = parentFiber, layoutEffectTraversalFlags = layoutEffectTraversalFlags$jscomp$0;
          switch (finishedWork.tag) {
            case 0:
            case 11:
            case 14:
            case 15:
              commitHookEffectListUnmount(4, finishedWork, finishedWork.return);
              recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
              break;
            case 1:
              safelyDetachRef(finishedWork, finishedWork.return);
              var instance = finishedWork.stateNode;
              typeof instance.componentWillUnmount === "function" && safelyCallComponentWillUnmount(finishedWork, finishedWork.return, instance);
              recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
              break;
            case 27:
              supportsSingletons && (layoutEffectTraversalFlags & 2) !== 0 && releaseSingletonInstance(finishedWork.stateNode, finishedWork.type, finishedWork.memoizedProps);
            case 5:
              safelyDetachRef(finishedWork, finishedWork.return);
              finishedWork.tag !== 5 && finishedWork.tag !== 27 || commitFragmentInstanceDeletionEffects(finishedWork);
              recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
              break;
            case 6:
              commitFragmentInstanceDeletionEffects(finishedWork);
              break;
            case 26:
              safelyDetachRef(finishedWork, finishedWork.return);
              supportsResources && (instance = finishedWork.stateNode, finishedWork.memoizedState !== null || instance === null || offscreenSubtreeWasHidden || unmountHoistable(instance));
              recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
              break;
            case 22:
              finishedWork.memoizedState === null && recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
              break;
            case 30:
              safelyDetachRef(finishedWork, finishedWork.return);
              recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
              break;
            case 7:
              safelyDetachRef(finishedWork, finishedWork.return);
            default:
              recursivelyTraverseDisappearLayoutEffects(finishedWork, layoutEffectTraversalFlags);
          }
          parentFiber = parentFiber.sibling;
        }
      }
      function recursivelyTraverseReappearLayoutEffects(finishedRoot$jscomp$0, parentFiber, layoutEffectTraversalFlags) {
        layoutEffectTraversalFlags = (parentFiber.subtreeFlags & 8772) !== 0 ? layoutEffectTraversalFlags : layoutEffectTraversalFlags & -2;
        for (parentFiber = parentFiber.child;parentFiber !== null; ) {
          var current = parentFiber.alternate, finishedRoot = finishedRoot$jscomp$0, finishedWork = parentFiber, flags = finishedWork.flags, includeWorkInProgressEffects = (layoutEffectTraversalFlags & 1) !== 0;
          switch (finishedWork.tag) {
            case 0:
            case 11:
            case 15:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              commitHookEffectListMount(4, finishedWork);
              break;
            case 1:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              current = finishedWork;
              finishedRoot = current.stateNode;
              if (typeof finishedRoot.componentDidMount === "function")
                try {
                  finishedRoot.componentDidMount();
                } catch (error) {
                  captureCommitPhaseError(current, current.return, error);
                }
              current = finishedWork;
              finishedRoot = current.updateQueue;
              if (finishedRoot !== null) {
                var instance = current.stateNode;
                try {
                  var hiddenCallbacks = finishedRoot.shared.hiddenCallbacks;
                  if (hiddenCallbacks !== null)
                    for (finishedRoot.shared.hiddenCallbacks = null, finishedRoot = 0;finishedRoot < hiddenCallbacks.length; finishedRoot++)
                      callCallback(hiddenCallbacks[finishedRoot], instance);
                } catch (error) {
                  captureCommitPhaseError(current, current.return, error);
                }
              }
              includeWorkInProgressEffects && flags & 64 && commitClassCallbacks(finishedWork);
              safelyAttachRef(finishedWork, finishedWork.return);
              break;
            case 27:
              supportsSingletons && (layoutEffectTraversalFlags & 2) !== 0 && commitHostSingletonAcquisition(finishedWork);
            case 5:
              finishedWork.tag !== 5 && finishedWork.tag !== 27 || commitFragmentInstanceInsertionEffects(finishedWork);
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              includeWorkInProgressEffects && current === null && flags & 4 && commitHostMount(finishedWork);
              safelyAttachRef(finishedWork, finishedWork.return);
              break;
            case 6:
              commitFragmentInstanceInsertionEffects(finishedWork);
              break;
            case 26:
              supportsResources && (instance = finishedWork.stateNode, finishedWork.memoizedState !== null || instance === null || offscreenSubtreeIsHidden || mountHoistable(getHoistableRoot(instance.ownerDocument), finishedWork.type, instance));
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              includeWorkInProgressEffects && current === null && flags & 4 && commitHostMount(finishedWork);
              safelyAttachRef(finishedWork, finishedWork.return);
              break;
            case 12:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              break;
            case 31:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              includeWorkInProgressEffects && flags & 4 && commitActivityHydrationCallbacks(finishedRoot, finishedWork);
              break;
            case 13:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              includeWorkInProgressEffects && flags & 4 && commitSuspenseHydrationCallbacks(finishedRoot, finishedWork);
              break;
            case 22:
              finishedWork.memoizedState === null && recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              safelyAttachRef(finishedWork, finishedWork.return);
              break;
            case 30:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
              safelyAttachRef(finishedWork, finishedWork.return);
              break;
            case 7:
              safelyAttachRef(finishedWork, finishedWork.return);
            default:
              recursivelyTraverseReappearLayoutEffects(finishedRoot, finishedWork, layoutEffectTraversalFlags);
          }
          parentFiber = parentFiber.sibling;
        }
      }
      function commitOffscreenPassiveMountEffects(current, finishedWork) {
        var previousCache = null;
        current !== null && current.memoizedState !== null && current.memoizedState.cachePool !== null && (previousCache = current.memoizedState.cachePool.pool);
        current = null;
        finishedWork.memoizedState !== null && finishedWork.memoizedState.cachePool !== null && (current = finishedWork.memoizedState.cachePool.pool);
        current !== previousCache && (current != null && current.refCount++, previousCache != null && releaseCache(previousCache));
      }
      function commitCachePassiveMountEffect(current, finishedWork) {
        current = null;
        finishedWork.alternate !== null && (current = finishedWork.alternate.memoizedState.cache);
        finishedWork = finishedWork.memoizedState.cache;
        finishedWork !== current && (finishedWork.refCount++, current != null && releaseCache(current));
      }
      function recursivelyTraversePassiveMountEffects(root, parentFiber, committedLanes, committedTransitions) {
        var isViewTransitionEligible = (committedLanes & 335544064) === committedLanes;
        if (parentFiber.subtreeFlags & (isViewTransitionEligible ? 10262 : 10256))
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            commitPassiveMountOnFiber(root, parentFiber, committedLanes, committedTransitions), parentFiber = parentFiber.sibling;
        else
          isViewTransitionEligible && restoreNestedViewTransitions(parentFiber);
      }
      function commitPassiveMountOnFiber(finishedRoot, finishedWork, committedLanes, committedTransitions) {
        var isViewTransitionEligible = (committedLanes & 335544064) === committedLanes;
        isViewTransitionEligible && finishedWork.alternate === null && finishedWork.return !== null && finishedWork.return.alternate !== null && restoreEnterOrExitViewTransitions(finishedWork);
        var flags = finishedWork.flags;
        switch (finishedWork.tag) {
          case 0:
          case 11:
          case 15:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            flags & 2048 && commitHookEffectListMount(9, finishedWork);
            break;
          case 1:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            break;
          case 3:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            isViewTransitionEligible && supportsMutation && rootViewTransitionNameCanceled && restoreRootViewTransitionName(finishedRoot.containerInfo);
            flags & 2048 && (finishedRoot = null, finishedWork.alternate !== null && (finishedRoot = finishedWork.alternate.memoizedState.cache), finishedWork = finishedWork.memoizedState.cache, finishedWork !== finishedRoot && (finishedWork.refCount++, finishedRoot != null && releaseCache(finishedRoot)));
            break;
          case 12:
            if (flags & 2048) {
              recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
              finishedRoot = finishedWork.stateNode;
              try {
                var _finishedWork$memoize2 = finishedWork.memoizedProps, id = _finishedWork$memoize2.id, onPostCommit = _finishedWork$memoize2.onPostCommit;
                typeof onPostCommit === "function" && onPostCommit(id, finishedWork.alternate === null ? "mount" : "update", finishedRoot.passiveEffectDuration, -0);
              } catch (error) {
                captureCommitPhaseError(finishedWork, finishedWork.return, error);
              }
            } else
              recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            break;
          case 31:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            break;
          case 13:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            break;
          case 23:
            break;
          case 22:
            _finishedWork$memoize2 = finishedWork.stateNode;
            id = finishedWork.alternate;
            finishedWork.memoizedState !== null ? (isViewTransitionEligible && id !== null && id.memoizedState === null && restoreEnterOrExitViewTransitions(id), _finishedWork$memoize2._visibility & 2 ? recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions) : recursivelyTraverseAtomicPassiveEffects(finishedRoot, finishedWork)) : (isViewTransitionEligible && id !== null && id.memoizedState !== null && restoreEnterOrExitViewTransitions(finishedWork), _finishedWork$memoize2._visibility & 2 ? recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions) : (_finishedWork$memoize2._visibility |= 2, recursivelyTraverseReconnectPassiveEffects(finishedRoot, finishedWork, committedLanes, committedTransitions, (finishedWork.subtreeFlags & 10256) !== 0 || false)));
            flags & 2048 && commitOffscreenPassiveMountEffects(id, finishedWork);
            break;
          case 24:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            flags & 2048 && commitCachePassiveMountEffect(finishedWork.alternate, finishedWork);
            break;
          case 30:
            isViewTransitionEligible && (isViewTransitionEligible = finishedWork.alternate, isViewTransitionEligible !== null && (restoreViewTransitionOnHostInstances(isViewTransitionEligible.child, true), restoreViewTransitionOnHostInstances(finishedWork.child, true)));
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
            break;
          default:
            recursivelyTraversePassiveMountEffects(finishedRoot, finishedWork, committedLanes, committedTransitions);
        }
      }
      function recursivelyTraverseReconnectPassiveEffects(finishedRoot$jscomp$0, parentFiber, committedLanes$jscomp$0, committedTransitions$jscomp$0, includeWorkInProgressEffects) {
        includeWorkInProgressEffects = includeWorkInProgressEffects && ((parentFiber.subtreeFlags & 10256) !== 0 || false);
        for (parentFiber = parentFiber.child;parentFiber !== null; ) {
          var finishedRoot = finishedRoot$jscomp$0, finishedWork = parentFiber, committedLanes = committedLanes$jscomp$0, committedTransitions = committedTransitions$jscomp$0, flags = finishedWork.flags;
          switch (finishedWork.tag) {
            case 0:
            case 11:
            case 15:
              recursivelyTraverseReconnectPassiveEffects(finishedRoot, finishedWork, committedLanes, committedTransitions, includeWorkInProgressEffects);
              commitHookEffectListMount(8, finishedWork);
              break;
            case 23:
              break;
            case 22:
              var instance = finishedWork.stateNode;
              finishedWork.memoizedState !== null ? instance._visibility & 2 ? recursivelyTraverseReconnectPassiveEffects(finishedRoot, finishedWork, committedLanes, committedTransitions, includeWorkInProgressEffects) : recursivelyTraverseAtomicPassiveEffects(finishedRoot, finishedWork) : (instance._visibility |= 2, recursivelyTraverseReconnectPassiveEffects(finishedRoot, finishedWork, committedLanes, committedTransitions, includeWorkInProgressEffects));
              includeWorkInProgressEffects && flags & 2048 && commitOffscreenPassiveMountEffects(finishedWork.alternate, finishedWork);
              break;
            case 24:
              recursivelyTraverseReconnectPassiveEffects(finishedRoot, finishedWork, committedLanes, committedTransitions, includeWorkInProgressEffects);
              includeWorkInProgressEffects && flags & 2048 && commitCachePassiveMountEffect(finishedWork.alternate, finishedWork);
              break;
            default:
              recursivelyTraverseReconnectPassiveEffects(finishedRoot, finishedWork, committedLanes, committedTransitions, includeWorkInProgressEffects);
          }
          parentFiber = parentFiber.sibling;
        }
      }
      function recursivelyTraverseAtomicPassiveEffects(finishedRoot$jscomp$0, parentFiber) {
        if (parentFiber.subtreeFlags & 10256)
          for (parentFiber = parentFiber.child;parentFiber !== null; ) {
            var finishedRoot = finishedRoot$jscomp$0, finishedWork = parentFiber, flags = finishedWork.flags;
            switch (finishedWork.tag) {
              case 22:
                recursivelyTraverseAtomicPassiveEffects(finishedRoot, finishedWork);
                flags & 2048 && commitOffscreenPassiveMountEffects(finishedWork.alternate, finishedWork);
                break;
              case 24:
                recursivelyTraverseAtomicPassiveEffects(finishedRoot, finishedWork);
                flags & 2048 && commitCachePassiveMountEffect(finishedWork.alternate, finishedWork);
                break;
              default:
                recursivelyTraverseAtomicPassiveEffects(finishedRoot, finishedWork);
            }
            parentFiber = parentFiber.sibling;
          }
      }
      function recursivelyAccumulateSuspenseyCommit(parentFiber, committedLanes, suspendedState) {
        if (parentFiber.subtreeFlags & suspenseyCommitFlag)
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            accumulateSuspenseyCommitOnFiber(parentFiber, committedLanes, suspendedState), parentFiber = parentFiber.sibling;
      }
      function accumulateSuspenseyCommitOnFiber(fiber, committedLanes, suspendedState) {
        switch (fiber.tag) {
          case 26:
            recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState);
            if (fiber.flags & suspenseyCommitFlag)
              if (fiber.memoizedState !== null)
                suspendResource(suspendedState, currentHoistableRoot, fiber.memoizedState, fiber.memoizedProps);
              else {
                var instance = fiber.stateNode, type = fiber.type;
                fiber = fiber.memoizedProps;
                ((committedLanes & 335544128) === committedLanes || maySuspendCommitInSyncRender(type, fiber)) && suspendInstance(suspendedState, instance, type, fiber);
              }
            break;
          case 5:
            recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState);
            fiber.flags & suspenseyCommitFlag && (instance = fiber.stateNode, type = fiber.type, fiber = fiber.memoizedProps, ((committedLanes & 335544128) === committedLanes || maySuspendCommitInSyncRender(type, fiber)) && suspendInstance(suspendedState, instance, type, fiber));
            break;
          case 3:
          case 4:
            supportsResources ? (instance = currentHoistableRoot, currentHoistableRoot = getHoistableRoot(fiber.stateNode.containerInfo), recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState), currentHoistableRoot = instance) : recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState);
            break;
          case 22:
            fiber.memoizedState === null && (instance = fiber.alternate, instance !== null && instance.memoizedState !== null ? (instance = suspenseyCommitFlag, suspenseyCommitFlag = 16777216, recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState), suspenseyCommitFlag = instance) : recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState));
            break;
          case 30:
            (fiber.flags & suspenseyCommitFlag) !== 0 && (instance = fiber.memoizedProps.name, instance != null && instance !== "auto" && (type = fiber.stateNode, type.paired = null, appearingViewTransitions === null && (appearingViewTransitions = new Map), appearingViewTransitions.set(instance, type)));
            recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState);
            break;
          default:
            recursivelyAccumulateSuspenseyCommit(fiber, committedLanes, suspendedState);
        }
      }
      function detachAlternateSiblings(parentFiber) {
        var previousFiber = parentFiber.alternate;
        if (previousFiber !== null && (parentFiber = previousFiber.child, parentFiber !== null)) {
          previousFiber.child = null;
          do
            previousFiber = parentFiber.sibling, parentFiber.sibling = null, parentFiber = previousFiber;
          while (parentFiber !== null);
        }
      }
      function recursivelyTraversePassiveUnmountEffects(parentFiber) {
        var deletions = parentFiber.deletions;
        if ((parentFiber.flags & 16) !== 0) {
          if (deletions !== null)
            for (var i = 0;i < deletions.length; i++) {
              var childToDelete = deletions[i];
              nextEffect = childToDelete;
              commitPassiveUnmountEffectsInsideOfDeletedTree_begin(childToDelete, parentFiber);
            }
          detachAlternateSiblings(parentFiber);
        }
        if (parentFiber.subtreeFlags & 10256)
          for (parentFiber = parentFiber.child;parentFiber !== null; )
            commitPassiveUnmountOnFiber(parentFiber), parentFiber = parentFiber.sibling;
      }
      function commitPassiveUnmountOnFiber(finishedWork) {
        switch (finishedWork.tag) {
          case 0:
          case 11:
          case 15:
            recursivelyTraversePassiveUnmountEffects(finishedWork);
            finishedWork.flags & 2048 && commitHookEffectListUnmount(9, finishedWork, finishedWork.return);
            break;
          case 3:
            recursivelyTraversePassiveUnmountEffects(finishedWork);
            break;
          case 12:
            recursivelyTraversePassiveUnmountEffects(finishedWork);
            break;
          case 22:
            var instance = finishedWork.stateNode;
            finishedWork.memoizedState !== null && instance._visibility & 2 && (finishedWork.return === null || finishedWork.return.tag !== 13) ? (instance._visibility &= -3, recursivelyTraverseDisconnectPassiveEffects(finishedWork)) : recursivelyTraversePassiveUnmountEffects(finishedWork);
            break;
          default:
            recursivelyTraversePassiveUnmountEffects(finishedWork);
        }
      }
      function recursivelyTraverseDisconnectPassiveEffects(parentFiber) {
        var deletions = parentFiber.deletions;
        if ((parentFiber.flags & 16) !== 0) {
          if (deletions !== null)
            for (var i = 0;i < deletions.length; i++) {
              var childToDelete = deletions[i];
              nextEffect = childToDelete;
              commitPassiveUnmountEffectsInsideOfDeletedTree_begin(childToDelete, parentFiber);
            }
          detachAlternateSiblings(parentFiber);
        }
        for (parentFiber = parentFiber.child;parentFiber !== null; ) {
          deletions = parentFiber;
          switch (deletions.tag) {
            case 0:
            case 11:
            case 15:
              commitHookEffectListUnmount(8, deletions, deletions.return);
              recursivelyTraverseDisconnectPassiveEffects(deletions);
              break;
            case 22:
              i = deletions.stateNode;
              i._visibility & 2 && (i._visibility &= -3, recursivelyTraverseDisconnectPassiveEffects(deletions));
              break;
            default:
              recursivelyTraverseDisconnectPassiveEffects(deletions);
          }
          parentFiber = parentFiber.sibling;
        }
      }
      function commitPassiveUnmountEffectsInsideOfDeletedTree_begin(deletedSubtreeRoot, nearestMountedAncestor) {
        for (;nextEffect !== null; ) {
          var fiber = nextEffect;
          switch (fiber.tag) {
            case 0:
            case 11:
            case 15:
              commitHookEffectListUnmount(8, fiber, nearestMountedAncestor);
              break;
            case 23:
            case 22:
              if (fiber.memoizedState !== null && fiber.memoizedState.cachePool !== null) {
                var cache = fiber.memoizedState.cachePool.pool;
                cache != null && cache.refCount++;
              }
              break;
            case 24:
              releaseCache(fiber.memoizedState.cache);
          }
          cache = fiber.child;
          if (cache !== null)
            cache.return = fiber, nextEffect = cache;
          else
            a:
              for (fiber = deletedSubtreeRoot;nextEffect !== null; ) {
                cache = nextEffect;
                var sibling = cache.sibling, returnFiber = cache.return;
                detachFiberAfterEffects(cache);
                if (cache === fiber) {
                  nextEffect = null;
                  break a;
                }
                if (sibling !== null) {
                  sibling.return = returnFiber;
                  nextEffect = sibling;
                  break a;
                }
                nextEffect = returnFiber;
              }
        }
      }
      function findFiberRootForHostRoot(hostRoot) {
        var maybeFiber = getInstanceFromNode(hostRoot);
        if (maybeFiber != null) {
          if (typeof maybeFiber.memoizedProps["data-testname"] !== "string")
            throw Error(formatProdErrorMessage(364));
          return maybeFiber;
        }
        hostRoot = findFiberRoot(hostRoot);
        if (hostRoot === null)
          throw Error(formatProdErrorMessage(362));
        return hostRoot.stateNode.current;
      }
      function matchSelector(fiber$jscomp$0, selector) {
        var tag = fiber$jscomp$0.tag;
        switch (selector.$$typeof) {
          case COMPONENT_TYPE:
            if (fiber$jscomp$0.type === selector.value)
              return true;
            break;
          case HAS_PSEUDO_CLASS_TYPE:
            a: {
              selector = selector.value;
              fiber$jscomp$0 = [fiber$jscomp$0, 0];
              for (tag = 0;tag < fiber$jscomp$0.length; ) {
                var fiber = fiber$jscomp$0[tag++], tag$jscomp$0 = fiber.tag, selectorIndex = fiber$jscomp$0[tag++], selector$jscomp$0 = selector[selectorIndex];
                if (tag$jscomp$0 !== 5 && tag$jscomp$0 !== 26 && tag$jscomp$0 !== 27 || !isHiddenSubtree(fiber)) {
                  for (;selector$jscomp$0 != null && matchSelector(fiber, selector$jscomp$0); )
                    selectorIndex++, selector$jscomp$0 = selector[selectorIndex];
                  if (selectorIndex === selector.length) {
                    selector = true;
                    break a;
                  } else
                    for (fiber = fiber.child;fiber !== null; )
                      fiber$jscomp$0.push(fiber, selectorIndex), fiber = fiber.sibling;
                }
              }
              selector = false;
            }
            return selector;
          case ROLE_TYPE:
            if ((tag === 5 || tag === 26 || tag === 27) && matchAccessibilityRole(fiber$jscomp$0.stateNode, selector.value))
              return true;
            break;
          case TEXT_TYPE:
            if (tag === 5 || tag === 6 || tag === 26 || tag === 27) {
              if (fiber$jscomp$0 = getTextContent(fiber$jscomp$0), fiber$jscomp$0 !== null && 0 <= fiber$jscomp$0.indexOf(selector.value))
                return true;
            }
            break;
          case TEST_NAME_TYPE:
            if (tag === 5 || tag === 26 || tag === 27) {
              if (fiber$jscomp$0 = fiber$jscomp$0.memoizedProps["data-testname"], typeof fiber$jscomp$0 === "string" && fiber$jscomp$0.toLowerCase() === selector.value.toLowerCase())
                return true;
            }
            break;
          default:
            throw Error(formatProdErrorMessage(365));
        }
        return false;
      }
      function selectorToString(selector) {
        switch (selector.$$typeof) {
          case COMPONENT_TYPE:
            return "<" + (getComponentNameFromType(selector.value) || "Unknown") + ">";
          case HAS_PSEUDO_CLASS_TYPE:
            return ":has(" + (selectorToString(selector) || "") + ")";
          case ROLE_TYPE:
            return '[role="' + selector.value + '"]';
          case TEXT_TYPE:
            return '"' + selector.value + '"';
          case TEST_NAME_TYPE:
            return '[data-testname="' + selector.value + '"]';
          default:
            throw Error(formatProdErrorMessage(365));
        }
      }
      function findPaths(root, selectors) {
        var matchingFibers = [];
        root = [root, 0];
        for (var index = 0;index < root.length; ) {
          var fiber = root[index++], tag = fiber.tag, selectorIndex = root[index++], selector = selectors[selectorIndex];
          if (tag !== 5 && tag !== 26 && tag !== 27 || !isHiddenSubtree(fiber)) {
            for (;selector != null && matchSelector(fiber, selector); )
              selectorIndex++, selector = selectors[selectorIndex];
            if (selectorIndex === selectors.length)
              matchingFibers.push(fiber);
            else
              for (fiber = fiber.child;fiber !== null; )
                root.push(fiber, selectorIndex), fiber = fiber.sibling;
          }
        }
        return matchingFibers;
      }
      function findAllNodes(hostRoot, selectors) {
        if (!supportsTestSelectors)
          throw Error(formatProdErrorMessage(363));
        hostRoot = findFiberRootForHostRoot(hostRoot);
        hostRoot = findPaths(hostRoot, selectors);
        selectors = [];
        hostRoot = Array.from(hostRoot);
        for (var index = 0;index < hostRoot.length; ) {
          var node = hostRoot[index++], tag = node.tag;
          if (tag === 5 || tag === 26 || tag === 27)
            isHiddenSubtree(node) || selectors.push(node.stateNode);
          else
            for (node = node.child;node !== null; )
              hostRoot.push(node), node = node.sibling;
        }
        return selectors;
      }
      function requestUpdateLane() {
        return (executionContext & 2) !== 0 && workInProgressRootRenderLanes !== 0 ? workInProgressRootRenderLanes & -workInProgressRootRenderLanes : ReactSharedInternals.T !== null ? requestTransitionLane() : resolveUpdatePriority();
      }
      function requestDeferredLane() {
        if (workInProgressDeferredLane === 0)
          if ((workInProgressRootRenderLanes & 536870912) === 0 || isHydrating) {
            var lane = nextTransitionDeferredLane;
            nextTransitionDeferredLane <<= 1;
            (nextTransitionDeferredLane & 3932160) === 0 && (nextTransitionDeferredLane = 262144);
            workInProgressDeferredLane = lane;
          } else
            workInProgressDeferredLane = 536870912;
        lane = suspenseHandlerStackCursor.current;
        lane !== null && (lane.flags |= 32);
        return workInProgressDeferredLane;
      }
      function scheduleViewTransitionEvent(fiber, callback) {
        if (callback != null) {
          var state = fiber.stateNode, instance = state.ref;
          instance === null && (instance = state.ref = createViewTransitionInstance(getViewTransitionName(fiber.memoizedProps, state)));
          pendingViewTransitionEvents === null && (pendingViewTransitionEvents = []);
          pendingViewTransitionEvents.push(callback.bind(null, instance));
        }
      }
      function scheduleUpdateOnFiber(root, fiber, lane) {
        if (root === workInProgressRoot && (workInProgressSuspendedReason === 2 || workInProgressSuspendedReason === 9) || root.cancelPendingCommit !== null)
          prepareFreshStack(root, 0), markRootSuspended(root, workInProgressRootRenderLanes, workInProgressDeferredLane, false);
        markRootUpdated$1(root, lane);
        if ((executionContext & 2) === 0 || root !== workInProgressRoot)
          root === workInProgressRoot && ((executionContext & 2) === 0 && (workInProgressRootInterleavedUpdatedLanes |= lane), workInProgressRootExitStatus === 4 && markRootSuspended(root, workInProgressRootRenderLanes, workInProgressDeferredLane, false)), ensureRootIsScheduled(root);
      }
      function performWorkOnRoot(root$jscomp$0, lanes, forceSync) {
        if ((executionContext & 6) !== 0)
          throw Error(formatProdErrorMessage(327));
        var shouldTimeSlice = !forceSync && (lanes & 127) === 0 && (lanes & root$jscomp$0.expiredLanes) === 0 || checkIfRootIsPrerendering(root$jscomp$0, lanes), exitStatus = shouldTimeSlice ? renderRootConcurrent(root$jscomp$0, lanes) : renderRootSync(root$jscomp$0, lanes, true), renderWasConcurrent = shouldTimeSlice;
        do {
          if (exitStatus === 0) {
            workInProgressRootIsPrerendering && !shouldTimeSlice && markRootSuspended(root$jscomp$0, lanes, 0, false);
            break;
          } else {
            forceSync = root$jscomp$0.current.alternate;
            if (renderWasConcurrent && !isRenderConsistentWithExternalStores(forceSync)) {
              exitStatus = renderRootSync(root$jscomp$0, lanes, false);
              renderWasConcurrent = false;
              continue;
            }
            if (exitStatus === 2) {
              renderWasConcurrent = lanes;
              if (root$jscomp$0.errorRecoveryDisabledLanes & renderWasConcurrent)
                var JSCompiler_inline_result = 0;
              else
                JSCompiler_inline_result = root$jscomp$0.pendingLanes & -536870913, JSCompiler_inline_result = JSCompiler_inline_result !== 0 ? JSCompiler_inline_result : JSCompiler_inline_result & 536870912 ? 536870912 : 0;
              if (JSCompiler_inline_result !== 0) {
                lanes = JSCompiler_inline_result;
                a: {
                  var root = root$jscomp$0;
                  exitStatus = workInProgressRootConcurrentErrors;
                  var wasRootDehydrated = supportsHydration && root.current.memoizedState.isDehydrated;
                  wasRootDehydrated && (prepareFreshStack(root, JSCompiler_inline_result).flags |= 256);
                  JSCompiler_inline_result = renderRootSync(root, JSCompiler_inline_result, false);
                  if (JSCompiler_inline_result !== 2 && JSCompiler_inline_result !== 6) {
                    if (workInProgressRootDidAttachPingListener && !wasRootDehydrated) {
                      root.errorRecoveryDisabledLanes |= renderWasConcurrent;
                      workInProgressRootInterleavedUpdatedLanes |= renderWasConcurrent;
                      exitStatus = 4;
                      break a;
                    }
                    renderWasConcurrent = workInProgressRootRecoverableErrors;
                    workInProgressRootRecoverableErrors = exitStatus;
                    renderWasConcurrent !== null && (workInProgressRootRecoverableErrors === null ? workInProgressRootRecoverableErrors = renderWasConcurrent : workInProgressRootRecoverableErrors.push.apply(workInProgressRootRecoverableErrors, renderWasConcurrent));
                  }
                  exitStatus = JSCompiler_inline_result;
                }
                renderWasConcurrent = false;
                if (exitStatus !== 2)
                  continue;
              }
            }
            if (exitStatus === 1) {
              prepareFreshStack(root$jscomp$0, 0);
              markRootSuspended(root$jscomp$0, lanes, 0, true);
              break;
            }
            a: {
              shouldTimeSlice = root$jscomp$0;
              renderWasConcurrent = exitStatus;
              switch (renderWasConcurrent) {
                case 0:
                case 1:
                  throw Error(formatProdErrorMessage(345));
                case 4:
                  if ((lanes & 4194048) !== lanes && (lanes & 62914560) !== lanes)
                    break;
                case 6:
                  markRootSuspended(shouldTimeSlice, lanes, workInProgressDeferredLane, !workInProgressRootDidSkipSuspendedSiblings);
                  break a;
                case 2:
                  workInProgressRootRecoverableErrors = null;
                  break;
                case 3:
                case 5:
                  break;
                default:
                  throw Error(formatProdErrorMessage(329));
              }
              if ((lanes & 62914560) === lanes && (exitStatus = globalMostRecentFallbackTime + 300 - now(), 10 < exitStatus)) {
                markRootSuspended(shouldTimeSlice, lanes, workInProgressDeferredLane, !workInProgressRootDidSkipSuspendedSiblings);
                if (getNextLanes(shouldTimeSlice, 0, true) !== 0)
                  break a;
                pendingEffectsLanes = lanes;
                shouldTimeSlice.timeoutHandle = scheduleTimeout(completeRootWhenReady.bind(null, shouldTimeSlice, forceSync, workInProgressRootRecoverableErrors, workInProgressTransitions, workInProgressRootDidIncludeRecursiveRenderUpdate, lanes, workInProgressDeferredLane, workInProgressRootInterleavedUpdatedLanes, workInProgressSuspendedRetryLanes, workInProgressRootDidSkipSuspendedSiblings, renderWasConcurrent, "Throttled", -0, 0), exitStatus);
                break a;
              }
              completeRootWhenReady(shouldTimeSlice, forceSync, workInProgressRootRecoverableErrors, workInProgressTransitions, workInProgressRootDidIncludeRecursiveRenderUpdate, lanes, workInProgressDeferredLane, workInProgressRootInterleavedUpdatedLanes, workInProgressSuspendedRetryLanes, workInProgressRootDidSkipSuspendedSiblings, renderWasConcurrent, null, -0, 0);
            }
          }
          break;
        } while (1);
        ensureRootIsScheduled(root$jscomp$0);
      }
      function completeRootWhenReady(root, finishedWork, recoverableErrors, transitions, didIncludeRenderPhaseUpdate, lanes, spawnedLane, updatedLanes, suspendedRetryLanes, didSkipSuspendedSiblings, exitStatus, suspendedCommitReason, completedRenderStartTime, completedRenderEndTime) {
        root.timeoutHandle = noTimeout;
        var subtreeFlags = finishedWork.subtreeFlags, isViewTransitionEligible = (lanes & 335544064) === lanes;
        suspendedCommitReason = null;
        if (isViewTransitionEligible || subtreeFlags & 8192 || (subtreeFlags & 16785408) === 16785408) {
          if (suspendedCommitReason = startSuspendingCommit(), appearingViewTransitions = null, accumulateSuspenseyCommitOnFiber(finishedWork, lanes, suspendedCommitReason), isViewTransitionEligible && suspendOnActiveViewTransition(suspendedCommitReason, root.containerInfo), subtreeFlags = (lanes & 62914560) === lanes ? globalMostRecentFallbackTime - now() : (lanes & 4194048) === lanes ? globalMostRecentTransitionTime - now() : 0, subtreeFlags = waitForCommitToBeReady(suspendedCommitReason, subtreeFlags), subtreeFlags !== null) {
            pendingEffectsLanes = lanes;
            root.cancelPendingCommit = subtreeFlags(completeRoot.bind(null, root, finishedWork, lanes, recoverableErrors, transitions, didIncludeRenderPhaseUpdate, spawnedLane, updatedLanes, suspendedRetryLanes, didSkipSuspendedSiblings, exitStatus, suspendedCommitReason, null, completedRenderStartTime, completedRenderEndTime));
            markRootSuspended(root, lanes, spawnedLane, !didSkipSuspendedSiblings);
            return;
          }
        }
        completeRoot(root, finishedWork, lanes, recoverableErrors, transitions, didIncludeRenderPhaseUpdate, spawnedLane, updatedLanes, suspendedRetryLanes, didSkipSuspendedSiblings, exitStatus, suspendedCommitReason);
      }
      function isRenderConsistentWithExternalStores(finishedWork) {
        for (var node = finishedWork;; ) {
          var tag = node.tag;
          if ((tag === 0 || tag === 11 || tag === 15) && node.flags & 16384 && (tag = node.updateQueue, tag !== null && (tag = tag.stores, tag !== null)))
            for (var i = 0;i < tag.length; i++) {
              var check = tag[i], getSnapshot = check.getSnapshot;
              check = check.value;
              try {
                if (!objectIs(getSnapshot(), check))
                  return false;
              } catch (error) {
                return false;
              }
            }
          tag = node.child;
          if (node.subtreeFlags & 16384 && tag !== null)
            tag.return = node, node = tag;
          else {
            if (node === finishedWork)
              break;
            for (;node.sibling === null; ) {
              if (node.return === null || node.return === finishedWork)
                return true;
              node = node.return;
            }
            node.sibling.return = node.return;
            node = node.sibling;
          }
        }
        return true;
      }
      function markRootSuspended(root, suspendedLanes, spawnedLane, didAttemptEntireTree) {
        suspendedLanes = getEntangledLanes(root, suspendedLanes);
        suspendedLanes &= ~workInProgressRootPingedLanes;
        suspendedLanes &= ~workInProgressRootInterleavedUpdatedLanes;
        root.suspendedLanes |= suspendedLanes;
        root.pingedLanes &= ~suspendedLanes;
        didAttemptEntireTree && (root.warmLanes |= suspendedLanes);
        didAttemptEntireTree = root.expirationTimes;
        for (var lanes = suspendedLanes;0 < lanes; ) {
          var index$4 = 31 - clz32(lanes), lane = 1 << index$4;
          didAttemptEntireTree[index$4] = -1;
          lanes &= ~lane;
        }
        spawnedLane !== 0 && markSpawnedDeferredLane(root, spawnedLane, suspendedLanes);
      }
      function flushSyncWork() {
        return (executionContext & 6) === 0 ? (flushSyncWorkAcrossRoots_impl(0, false), false) : true;
      }
      function resetWorkInProgressStack() {
        if (workInProgress !== null) {
          if (workInProgressSuspendedReason === 0)
            var interruptedWork = workInProgress.return;
          else
            interruptedWork = workInProgress, lastContextDependency = currentlyRenderingFiber$1 = null, resetHooksOnUnwind(interruptedWork), thenableState$1 = null, thenableIndexCounter$1 = 0, interruptedWork = workInProgress;
          for (;interruptedWork !== null; )
            unwindInterruptedWork(interruptedWork.alternate, interruptedWork), interruptedWork = interruptedWork.return;
          workInProgress = null;
        }
      }
      function prepareFreshStack(root, lanes) {
        var timeoutHandle = root.timeoutHandle;
        timeoutHandle !== noTimeout && (root.timeoutHandle = noTimeout, cancelTimeout(timeoutHandle));
        timeoutHandle = root.cancelPendingCommit;
        timeoutHandle !== null && (root.cancelPendingCommit = null, timeoutHandle());
        pendingEffectsLanes = 0;
        resetWorkInProgressStack();
        workInProgressRoot = root;
        workInProgress = timeoutHandle = createWorkInProgress(root.current, null);
        workInProgressRootRenderLanes = lanes;
        workInProgressSuspendedReason = 0;
        workInProgressThrownValue = null;
        workInProgressRootDidSkipSuspendedSiblings = false;
        workInProgressRootIsPrerendering = checkIfRootIsPrerendering(root, lanes);
        workInProgressRootDidAttachPingListener = false;
        workInProgressSuspendedRetryLanes = workInProgressDeferredLane = workInProgressRootPingedLanes = workInProgressRootInterleavedUpdatedLanes = workInProgressRootSkippedLanes = workInProgressRootExitStatus = 0;
        workInProgressRootRecoverableErrors = workInProgressRootConcurrentErrors = null;
        workInProgressRootDidIncludeRecursiveRenderUpdate = false;
        entangledRenderLanes = getEntangledLanes(root, lanes);
        finishQueueingConcurrentUpdates();
        return timeoutHandle;
      }
      function handleThrow(root, thrownValue) {
        currentlyRenderingFiber = null;
        ReactSharedInternals.H = ContextOnlyDispatcher;
        thrownValue === SuspenseException || thrownValue === SuspenseActionException ? (thrownValue = getSuspendedThenable(), workInProgressSuspendedReason = 3) : thrownValue === SuspenseyCommitException ? (thrownValue = getSuspendedThenable(), workInProgressSuspendedReason = 4) : workInProgressSuspendedReason = thrownValue === SelectiveHydrationException ? 8 : thrownValue !== null && typeof thrownValue === "object" && typeof thrownValue.then === "function" ? 6 : 1;
        workInProgressThrownValue = thrownValue;
        workInProgress === null && (workInProgressRootExitStatus = 1, logUncaughtError(root, createCapturedValueAtFiber(thrownValue, root.current)));
      }
      function shouldRemainOnPreviousScreen() {
        var handler = suspenseHandlerStackCursor.current;
        return handler === null ? true : (workInProgressRootRenderLanes & 4194048) === workInProgressRootRenderLanes ? shellBoundary === null ? true : false : (workInProgressRootRenderLanes & 62914560) === workInProgressRootRenderLanes || (workInProgressRootRenderLanes & 536870912) !== 0 ? handler === shellBoundary : false;
      }
      function pushDispatcher() {
        var prevDispatcher = ReactSharedInternals.H;
        ReactSharedInternals.H = ContextOnlyDispatcher;
        return prevDispatcher === null ? ContextOnlyDispatcher : prevDispatcher;
      }
      function pushAsyncDispatcher() {
        var prevAsyncDispatcher = ReactSharedInternals.A;
        ReactSharedInternals.A = DefaultAsyncDispatcher;
        return prevAsyncDispatcher;
      }
      function renderDidSuspendDelayIfPossible() {
        workInProgressRootExitStatus = 4;
        workInProgressRootDidSkipSuspendedSiblings || (workInProgressRootRenderLanes & 4194048) !== workInProgressRootRenderLanes && suspenseHandlerStackCursor.current !== null || (workInProgressRootIsPrerendering = true);
        (workInProgressRootSkippedLanes & 134217727) === 0 && (workInProgressRootInterleavedUpdatedLanes & 134217727) === 0 || workInProgressRoot === null || markRootSuspended(workInProgressRoot, workInProgressRootRenderLanes, workInProgressDeferredLane, false);
      }
      function renderRootSync(root, lanes, shouldYieldForPrerendering) {
        var prevExecutionContext = executionContext;
        executionContext |= 2;
        var prevDispatcher = pushDispatcher(), prevAsyncDispatcher = pushAsyncDispatcher();
        if (workInProgressRoot !== root || workInProgressRootRenderLanes !== lanes)
          workInProgressTransitions = null, prepareFreshStack(root, lanes);
        lanes = false;
        var exitStatus = workInProgressRootExitStatus;
        a:
          do
            try {
              if (workInProgressSuspendedReason !== 0 && workInProgress !== null) {
                var unitOfWork = workInProgress, thrownValue = workInProgressThrownValue;
                switch (workInProgressSuspendedReason) {
                  case 8:
                    resetWorkInProgressStack();
                    exitStatus = 6;
                    break a;
                  case 3:
                  case 2:
                  case 9:
                  case 6:
                    suspenseHandlerStackCursor.current === null && (lanes = true);
                    var reason = workInProgressSuspendedReason;
                    workInProgressSuspendedReason = 0;
                    workInProgressThrownValue = null;
                    throwAndUnwindWorkLoop(root, unitOfWork, thrownValue, reason);
                    if (shouldYieldForPrerendering && workInProgressRootIsPrerendering) {
                      exitStatus = 0;
                      break a;
                    }
                    break;
                  default:
                    reason = workInProgressSuspendedReason, workInProgressSuspendedReason = 0, workInProgressThrownValue = null, throwAndUnwindWorkLoop(root, unitOfWork, thrownValue, reason);
                }
              }
              workLoopSync();
              exitStatus = workInProgressRootExitStatus;
              break;
            } catch (thrownValue$169) {
              handleThrow(root, thrownValue$169);
            }
          while (1);
        lanes && root.shellSuspendCounter++;
        lastContextDependency = currentlyRenderingFiber$1 = null;
        executionContext = prevExecutionContext;
        ReactSharedInternals.H = prevDispatcher;
        ReactSharedInternals.A = prevAsyncDispatcher;
        workInProgress === null && (workInProgressRoot = null, workInProgressRootRenderLanes = 0, finishQueueingConcurrentUpdates());
        return exitStatus;
      }
      function workLoopSync() {
        for (;workInProgress !== null; )
          performUnitOfWork(workInProgress);
      }
      function renderRootConcurrent(root, lanes) {
        var prevExecutionContext = executionContext;
        executionContext |= 2;
        var prevDispatcher = pushDispatcher(), prevAsyncDispatcher = pushAsyncDispatcher();
        workInProgressRoot !== root || workInProgressRootRenderLanes !== lanes ? (workInProgressTransitions = null, workInProgressRootRenderTargetTime = now() + 500, prepareFreshStack(root, lanes)) : workInProgressRootIsPrerendering = checkIfRootIsPrerendering(root, lanes);
        a:
          do
            try {
              if (workInProgressSuspendedReason !== 0 && workInProgress !== null) {
                lanes = workInProgress;
                var thrownValue = workInProgressThrownValue;
                b:
                  switch (workInProgressSuspendedReason) {
                    case 1:
                      workInProgressSuspendedReason = 0;
                      workInProgressThrownValue = null;
                      throwAndUnwindWorkLoop(root, lanes, thrownValue, 1);
                      break;
                    case 2:
                    case 9:
                      if (isThenableResolved(thrownValue)) {
                        workInProgressSuspendedReason = 0;
                        workInProgressThrownValue = null;
                        replaySuspendedUnitOfWork(lanes);
                        break;
                      }
                      lanes = function() {
                        workInProgressSuspendedReason !== 2 && workInProgressSuspendedReason !== 9 || workInProgressRoot !== root || (workInProgressSuspendedReason = 7);
                        ensureRootIsScheduled(root);
                      };
                      thrownValue.then(lanes, lanes);
                      break a;
                    case 3:
                      workInProgressSuspendedReason = 7;
                      break a;
                    case 4:
                      workInProgressSuspendedReason = 5;
                      break a;
                    case 7:
                      isThenableResolved(thrownValue) ? (workInProgressSuspendedReason = 0, workInProgressThrownValue = null, replaySuspendedUnitOfWork(lanes)) : (workInProgressSuspendedReason = 0, workInProgressThrownValue = null, throwAndUnwindWorkLoop(root, lanes, thrownValue, 7));
                      break;
                    case 5:
                      var resource = null;
                      switch (workInProgress.tag) {
                        case 26:
                          resource = workInProgress.memoizedState;
                        case 5:
                        case 27:
                          var hostFiber = workInProgress, { type, pendingProps: props } = hostFiber;
                          if (resource ? preloadResource(resource) : preloadInstance(hostFiber.stateNode, type, props)) {
                            workInProgressSuspendedReason = 0;
                            workInProgressThrownValue = null;
                            var sibling = hostFiber.sibling;
                            if (sibling !== null)
                              workInProgress = sibling;
                            else {
                              var returnFiber = hostFiber.return;
                              returnFiber !== null ? (workInProgress = returnFiber, completeUnitOfWork(returnFiber)) : workInProgress = null;
                            }
                            break b;
                          }
                      }
                      workInProgressSuspendedReason = 0;
                      workInProgressThrownValue = null;
                      throwAndUnwindWorkLoop(root, lanes, thrownValue, 5);
                      break;
                    case 6:
                      workInProgressSuspendedReason = 0;
                      workInProgressThrownValue = null;
                      throwAndUnwindWorkLoop(root, lanes, thrownValue, 6);
                      break;
                    case 8:
                      resetWorkInProgressStack();
                      workInProgressRootExitStatus = 6;
                      break a;
                    default:
                      throw Error(formatProdErrorMessage(462));
                  }
              }
              workLoopConcurrentByScheduler();
              break;
            } catch (thrownValue$171) {
              handleThrow(root, thrownValue$171);
            }
          while (1);
        lastContextDependency = currentlyRenderingFiber$1 = null;
        ReactSharedInternals.H = prevDispatcher;
        ReactSharedInternals.A = prevAsyncDispatcher;
        executionContext = prevExecutionContext;
        if (workInProgress !== null)
          return 0;
        workInProgressRoot = null;
        workInProgressRootRenderLanes = 0;
        finishQueueingConcurrentUpdates();
        return workInProgressRootExitStatus;
      }
      function workLoopConcurrentByScheduler() {
        for (;workInProgress !== null && !shouldYield(); )
          performUnitOfWork(workInProgress);
      }
      function performUnitOfWork(unitOfWork) {
        var next = beginWork(unitOfWork.alternate, unitOfWork, entangledRenderLanes);
        unitOfWork.memoizedProps = unitOfWork.pendingProps;
        next === null ? completeUnitOfWork(unitOfWork) : workInProgress = next;
      }
      function replaySuspendedUnitOfWork(unitOfWork) {
        var next = unitOfWork;
        var current = next.alternate;
        switch (next.tag) {
          case 15:
          case 0:
            next = replayFunctionComponent(current, next, next.pendingProps, next.type, undefined, workInProgressRootRenderLanes);
            break;
          case 11:
            next = replayFunctionComponent(current, next, next.pendingProps, next.type.render, next.ref, workInProgressRootRenderLanes);
            break;
          case 5:
            resetHooksOnUnwind(next);
            var fiber = next;
            supportsHydration && fiber === hydrationParentFiber && (isHydrating ? (popToNextHostParent(fiber), fiber.tag === 5 && fiber.stateNode != null && (nextHydratableInstance = fiber.stateNode)) : (popToNextHostParent(fiber), isHydrating = true));
          default:
            unwindInterruptedWork(current, next), next = workInProgress = resetWorkInProgress(next, entangledRenderLanes), next = beginWork(current, next, entangledRenderLanes);
        }
        unitOfWork.memoizedProps = unitOfWork.pendingProps;
        next === null ? completeUnitOfWork(unitOfWork) : workInProgress = next;
      }
      function throwAndUnwindWorkLoop(root, unitOfWork, thrownValue, suspendedReason) {
        lastContextDependency = currentlyRenderingFiber$1 = null;
        resetHooksOnUnwind(unitOfWork);
        thenableState$1 = null;
        thenableIndexCounter$1 = 0;
        var returnFiber = unitOfWork.return;
        try {
          if (throwException(root, returnFiber, unitOfWork, thrownValue, workInProgressRootRenderLanes)) {
            workInProgressRootExitStatus = 1;
            logUncaughtError(root, createCapturedValueAtFiber(thrownValue, root.current));
            workInProgress = null;
            return;
          }
        } catch (error) {
          if (returnFiber !== null)
            throw workInProgress = returnFiber, error;
          workInProgressRootExitStatus = 1;
          logUncaughtError(root, createCapturedValueAtFiber(thrownValue, root.current));
          workInProgress = null;
          return;
        }
        if (unitOfWork.flags & 32768) {
          if (isHydrating || suspendedReason === 1)
            root = true;
          else if (workInProgressRootIsPrerendering || (workInProgressRootRenderLanes & 536870912) !== 0)
            root = false;
          else if (workInProgressRootDidSkipSuspendedSiblings = root = true, suspendedReason === 2 || suspendedReason === 9 || suspendedReason === 3 || suspendedReason === 6)
            suspendedReason = suspenseHandlerStackCursor.current, suspendedReason !== null && suspendedReason.tag === 13 && (suspendedReason.flags |= 16384);
          unwindUnitOfWork(unitOfWork, root);
        } else
          completeUnitOfWork(unitOfWork);
      }
      function completeUnitOfWork(unitOfWork) {
        var completedWork = unitOfWork;
        do {
          if ((completedWork.flags & 32768) !== 0) {
            unwindUnitOfWork(completedWork, workInProgressRootDidSkipSuspendedSiblings);
            return;
          }
          unitOfWork = completedWork.return;
          var next = completeWork(completedWork.alternate, completedWork, entangledRenderLanes);
          if (next !== null) {
            workInProgress = next;
            return;
          }
          completedWork = completedWork.sibling;
          if (completedWork !== null) {
            workInProgress = completedWork;
            return;
          }
          workInProgress = completedWork = unitOfWork;
        } while (completedWork !== null);
        workInProgressRootExitStatus === 0 && (workInProgressRootExitStatus = 5);
      }
      function unwindUnitOfWork(unitOfWork, skipSiblings) {
        do {
          var next = unwindWork(unitOfWork.alternate, unitOfWork);
          if (next !== null) {
            next.flags &= 32767;
            workInProgress = next;
            return;
          }
          next = unitOfWork.return;
          next !== null && (next.flags |= 32768, next.subtreeFlags = 0, next.deletions = null);
          if (!skipSiblings && (unitOfWork = unitOfWork.sibling, unitOfWork !== null)) {
            workInProgress = unitOfWork;
            return;
          }
          workInProgress = unitOfWork = next;
        } while (unitOfWork !== null);
        workInProgressRootExitStatus = 6;
        workInProgress = null;
      }
      function completeRoot(root, finishedWork, lanes, recoverableErrors, transitions, didIncludeRenderPhaseUpdate, spawnedLane, updatedLanes, suspendedRetryLanes, didSkipSuspendedSiblings, exitStatus, suspendedState) {
        root.cancelPendingCommit = null;
        do
          flushPendingEffects();
        while (pendingEffectsStatus !== 0);
        if ((executionContext & 6) !== 0)
          throw Error(formatProdErrorMessage(327));
        if (finishedWork !== null) {
          if (finishedWork === root.current)
            throw Error(formatProdErrorMessage(177));
          root === workInProgressRoot && (workInProgress = workInProgressRoot = null, workInProgressRootRenderLanes = 0);
          pendingFinishedWork = finishedWork;
          pendingEffectsRoot = root;
          pendingEffectsLanes = lanes;
          pendingPassiveTransitions = transitions;
          pendingRecoverableErrors = recoverableErrors;
          commitRoot(root, finishedWork, lanes, spawnedLane, updatedLanes, suspendedRetryLanes, suspendedState);
        }
      }
      function commitRoot(root, finishedWork, lanes, spawnedLane, updatedLanes, suspendedRetryLanes, suspendedState) {
        var remainingLanes = finishedWork.lanes | finishedWork.childLanes;
        pendingEffectsRemainingLanes = remainingLanes;
        remainingLanes |= concurrentlyUpdatedLanes;
        markRootFinished(root, lanes, remainingLanes, spawnedLane, updatedLanes, suspendedRetryLanes);
        pendingViewTransitionEvents = null;
        (lanes & 335544064) === lanes ? (pendingTransitionTypes = claimQueuedTransitionTypes(root), spawnedLane = 10262) : (pendingTransitionTypes = null, spawnedLane = 10256);
        (finishedWork.subtreeFlags & spawnedLane) !== 0 || (finishedWork.flags & spawnedLane) !== 0 ? (root.callbackNode = null, root.callbackPriority = 0, scheduleCallback(NormalPriority$1, function() {
          flushPassiveEffects();
          return null;
        })) : (root.callbackNode = null, root.callbackPriority = 0);
        shouldStartViewTransition = false;
        spawnedLane = (finishedWork.flags & 13878) !== 0;
        if ((finishedWork.subtreeFlags & 13878) !== 0 || spawnedLane) {
          spawnedLane = ReactSharedInternals.T;
          ReactSharedInternals.T = null;
          updatedLanes = getCurrentUpdatePriority();
          setCurrentUpdatePriority(2);
          suspendedRetryLanes = executionContext;
          executionContext |= 4;
          try {
            commitBeforeMutationEffects(root, finishedWork, lanes);
          } finally {
            executionContext = suspendedRetryLanes, setCurrentUpdatePriority(updatedLanes), ReactSharedInternals.T = spawnedLane;
          }
        }
        pendingEffectsStatus = 1;
        shouldStartViewTransition ? pendingViewTransition = startViewTransition(suspendedState, root.containerInfo, pendingTransitionTypes, flushMutationEffects, flushLayoutEffects, flushAfterMutationEffects, flushSpawnedWork, flushPassiveEffects, reportViewTransitionError, null, null) : (flushMutationEffects(), flushLayoutEffects(), flushSpawnedWork());
      }
      function reportViewTransitionError(error) {
        if (pendingEffectsStatus !== 0) {
          var onRecoverableError = pendingEffectsRoot.onRecoverableError;
          onRecoverableError(error, { componentStack: null });
        }
      }
      function flushAfterMutationEffects() {
        pendingEffectsStatus === 3 && (pendingEffectsStatus = 0, commitAfterMutationEffectsOnFiber(pendingFinishedWork, pendingEffectsRoot), pendingEffectsStatus = 4);
      }
      function flushMutationEffects() {
        if (pendingEffectsStatus === 1) {
          pendingEffectsStatus = 0;
          var root = pendingEffectsRoot, finishedWork = pendingFinishedWork, lanes = pendingEffectsLanes, rootMutationHasEffect = (finishedWork.flags & 13878) !== 0;
          if ((finishedWork.subtreeFlags & 13878) !== 0 || rootMutationHasEffect) {
            rootMutationHasEffect = ReactSharedInternals.T;
            ReactSharedInternals.T = null;
            var previousPriority = getCurrentUpdatePriority();
            setCurrentUpdatePriority(2);
            var prevExecutionContext = executionContext;
            executionContext |= 4;
            try {
              inUpdateViewTransition = rootViewTransitionAffected = false, commitMutationEffectsOnFiber(finishedWork, root, lanes), resetAfterCommit(root.containerInfo);
            } finally {
              executionContext = prevExecutionContext, setCurrentUpdatePriority(previousPriority), ReactSharedInternals.T = rootMutationHasEffect;
            }
          }
          root.current = finishedWork;
          pendingEffectsStatus = 2;
        }
      }
      function flushLayoutEffects() {
        if (pendingEffectsStatus === 2) {
          pendingEffectsStatus = 0;
          var root = pendingEffectsRoot, finishedWork = pendingFinishedWork, rootHasLayoutEffect = (finishedWork.flags & 8772) !== 0;
          if ((finishedWork.subtreeFlags & 8772) !== 0 || rootHasLayoutEffect) {
            rootHasLayoutEffect = ReactSharedInternals.T;
            ReactSharedInternals.T = null;
            var previousPriority = getCurrentUpdatePriority();
            setCurrentUpdatePriority(2);
            var prevExecutionContext = executionContext;
            executionContext |= 4;
            try {
              commitLayoutEffectOnFiber(root, finishedWork.alternate, finishedWork);
            } finally {
              executionContext = prevExecutionContext, setCurrentUpdatePriority(previousPriority), ReactSharedInternals.T = rootHasLayoutEffect;
            }
          }
          pendingEffectsStatus = 3;
        }
      }
      function flushSpawnedWork() {
        if (pendingEffectsStatus === 4 || pendingEffectsStatus === 3) {
          pendingEffectsStatus = 0;
          var committedViewTransition = pendingViewTransition;
          pendingViewTransition = null;
          requestPaint();
          var root = pendingEffectsRoot, finishedWork = pendingFinishedWork, lanes = pendingEffectsLanes, recoverableErrors = pendingRecoverableErrors, passiveSubtreeMask = (lanes & 335544064) === lanes ? 10262 : 10256;
          (finishedWork.subtreeFlags & passiveSubtreeMask) !== 0 || (finishedWork.flags & passiveSubtreeMask) !== 0 ? pendingEffectsStatus = 5 : (pendingEffectsStatus = 0, pendingFinishedWork = pendingEffectsRoot = null, releaseRootPooledCache(root, root.pendingLanes));
          passiveSubtreeMask = root.pendingLanes;
          passiveSubtreeMask === 0 && (legacyErrorBoundariesThatAlreadyFailed = null);
          lanesToEventPriority(lanes);
          finishedWork = finishedWork.stateNode;
          if (injectedHook && typeof injectedHook.onCommitFiberRoot === "function")
            try {
              injectedHook.onCommitFiberRoot(rendererID, finishedWork, undefined, (finishedWork.current.flags & 128) === 128);
            } catch (err) {}
          if (recoverableErrors !== null) {
            finishedWork = ReactSharedInternals.T;
            passiveSubtreeMask = getCurrentUpdatePriority();
            setCurrentUpdatePriority(2);
            ReactSharedInternals.T = null;
            try {
              for (var onRecoverableError = root.onRecoverableError, i = 0;i < recoverableErrors.length; i++) {
                var recoverableError = recoverableErrors[i];
                onRecoverableError(recoverableError.value, {
                  componentStack: recoverableError.stack
                });
              }
            } finally {
              ReactSharedInternals.T = finishedWork, setCurrentUpdatePriority(passiveSubtreeMask);
            }
          }
          recoverableErrors = pendingViewTransitionEvents;
          onRecoverableError = pendingTransitionTypes;
          pendingTransitionTypes = null;
          if (recoverableErrors !== null && (pendingViewTransitionEvents = null, onRecoverableError === null && (onRecoverableError = []), committedViewTransition !== null))
            for (recoverableError = 0;recoverableError < recoverableErrors.length; recoverableError++)
              finishedWork = (0, recoverableErrors[recoverableError])(onRecoverableError), finishedWork !== undefined && addViewTransitionFinishedListener(committedViewTransition, finishedWork);
          (pendingEffectsLanes & 3) !== 0 && flushPendingEffects();
          ensureRootIsScheduled(root);
          passiveSubtreeMask = root.pendingLanes;
          (lanes & 261930) !== 0 && (passiveSubtreeMask & 42) !== 0 ? root === rootWithNestedUpdates ? nestedUpdateCount++ : (nestedUpdateCount = 0, rootWithNestedUpdates = root) : (nestedUpdateCount = 0, rootWithNestedUpdates = null);
          supportsHydration && flushHydrationEvents();
          flushSyncWorkAcrossRoots_impl(0, false);
        }
      }
      function releaseRootPooledCache(root, remainingLanes) {
        (root.pooledCacheLanes &= remainingLanes) === 0 && (remainingLanes = root.pooledCache, remainingLanes != null && (root.pooledCache = null, releaseCache(remainingLanes)));
      }
      function flushPendingEffects() {
        pendingViewTransition !== null && (stopViewTransition(pendingViewTransition), pendingViewTransition = null);
        flushMutationEffects();
        flushLayoutEffects();
        flushSpawnedWork();
        return flushPassiveEffects();
      }
      function flushPassiveEffects() {
        if (pendingEffectsStatus !== 5)
          return false;
        var root = pendingEffectsRoot, remainingLanes = pendingEffectsRemainingLanes;
        pendingEffectsRemainingLanes = 0;
        var renderPriority = lanesToEventPriority(pendingEffectsLanes), priority = 32 > renderPriority ? 32 : renderPriority;
        renderPriority = ReactSharedInternals.T;
        var previousPriority = getCurrentUpdatePriority();
        try {
          setCurrentUpdatePriority(priority);
          ReactSharedInternals.T = null;
          priority = pendingPassiveTransitions;
          pendingPassiveTransitions = null;
          var root$jscomp$0 = pendingEffectsRoot, lanes = pendingEffectsLanes;
          pendingEffectsStatus = 0;
          pendingFinishedWork = pendingEffectsRoot = null;
          pendingEffectsLanes = 0;
          if ((executionContext & 6) !== 0)
            throw Error(formatProdErrorMessage(331));
          var prevExecutionContext = executionContext;
          executionContext |= 4;
          commitPassiveUnmountOnFiber(root$jscomp$0.current);
          commitPassiveMountOnFiber(root$jscomp$0, root$jscomp$0.current, lanes, priority);
          executionContext = prevExecutionContext;
          flushSyncWorkAcrossRoots_impl(0, false);
          if (injectedHook && typeof injectedHook.onPostCommitFiberRoot === "function")
            try {
              injectedHook.onPostCommitFiberRoot(rendererID, root$jscomp$0);
            } catch (err) {}
          return true;
        } finally {
          setCurrentUpdatePriority(previousPriority), ReactSharedInternals.T = renderPriority, releaseRootPooledCache(root, remainingLanes);
        }
      }
      function captureCommitPhaseErrorOnRoot(rootFiber, sourceFiber, error) {
        sourceFiber = createCapturedValueAtFiber(error, sourceFiber);
        sourceFiber = createRootErrorUpdate(rootFiber.stateNode, sourceFiber, 2);
        rootFiber = enqueueUpdate(rootFiber, sourceFiber, 2);
        rootFiber !== null && (markRootUpdated$1(rootFiber, 2), ensureRootIsScheduled(rootFiber));
      }
      function captureCommitPhaseError(sourceFiber, nearestMountedAncestor, error) {
        if (sourceFiber.tag === 3)
          captureCommitPhaseErrorOnRoot(sourceFiber, sourceFiber, error);
        else
          for (;nearestMountedAncestor !== null; ) {
            if (nearestMountedAncestor.tag === 3) {
              captureCommitPhaseErrorOnRoot(nearestMountedAncestor, sourceFiber, error);
              break;
            } else if (nearestMountedAncestor.tag === 1) {
              var instance = nearestMountedAncestor.stateNode;
              if (typeof nearestMountedAncestor.type.getDerivedStateFromError === "function" || typeof instance.componentDidCatch === "function" && (legacyErrorBoundariesThatAlreadyFailed === null || !legacyErrorBoundariesThatAlreadyFailed.has(instance))) {
                sourceFiber = createCapturedValueAtFiber(error, sourceFiber);
                error = createClassErrorUpdate(2);
                instance = enqueueUpdate(nearestMountedAncestor, error, 2);
                instance !== null && (initializeClassErrorUpdate(error, instance, nearestMountedAncestor, sourceFiber), markRootUpdated$1(instance, 2), ensureRootIsScheduled(instance));
                break;
              }
            }
            nearestMountedAncestor = nearestMountedAncestor.return;
          }
      }
      function attachPingListener(root, wakeable, lanes) {
        var pingCache = root.pingCache;
        if (pingCache === null) {
          pingCache = root.pingCache = new PossiblyWeakMap;
          var threadIDs = new Set;
          pingCache.set(wakeable, threadIDs);
        } else
          threadIDs = pingCache.get(wakeable), threadIDs === undefined && (threadIDs = new Set, pingCache.set(wakeable, threadIDs));
        threadIDs.has(lanes) || (workInProgressRootDidAttachPingListener = true, threadIDs.add(lanes), root = pingSuspendedRoot.bind(null, root, wakeable, lanes), wakeable.then(root, root));
      }
      function pingSuspendedRoot(root, wakeable, pingedLanes) {
        var pingCache = root.pingCache;
        pingCache !== null && pingCache.delete(wakeable);
        root.pingedLanes |= root.suspendedLanes & pingedLanes;
        root.warmLanes &= ~pingedLanes;
        workInProgressRoot === root && (workInProgressRootRenderLanes & pingedLanes) === pingedLanes && (workInProgressRootExitStatus === 4 || workInProgressRootExitStatus === 3 && (workInProgressRootRenderLanes & 62914560) === workInProgressRootRenderLanes && 300 > now() - globalMostRecentFallbackTime ? (executionContext & 2) === 0 ? prepareFreshStack(root, 0) : workInProgressRootPingedLanes |= pingedLanes : workInProgressRootPingedLanes |= pingedLanes, workInProgressSuspendedRetryLanes === workInProgressRootRenderLanes && (workInProgressSuspendedRetryLanes = 0));
        ensureRootIsScheduled(root);
      }
      function retryTimedOutBoundary(boundaryFiber, retryLane) {
        retryLane === 0 && (retryLane = claimNextRetryLane());
        boundaryFiber = enqueueConcurrentRenderForLane(boundaryFiber, retryLane);
        boundaryFiber !== null && (markRootUpdated$1(boundaryFiber, retryLane), ensureRootIsScheduled(boundaryFiber));
      }
      function retryDehydratedSuspenseBoundary(boundaryFiber) {
        var suspenseState = boundaryFiber.memoizedState, retryLane = 0;
        suspenseState !== null && (retryLane = suspenseState.retryLane);
        retryTimedOutBoundary(boundaryFiber, retryLane);
      }
      function resolveRetryWakeable(boundaryFiber, wakeable) {
        var retryLane = 0;
        switch (boundaryFiber.tag) {
          case 31:
          case 13:
            var retryCache = boundaryFiber.stateNode;
            var suspenseState = boundaryFiber.memoizedState;
            suspenseState !== null && (retryLane = suspenseState.retryLane);
            break;
          case 19:
            retryCache = boundaryFiber.stateNode;
            break;
          case 22:
            retryCache = boundaryFiber.stateNode._retryCache;
            break;
          default:
            throw Error(formatProdErrorMessage(314));
        }
        retryCache !== null && retryCache.delete(wakeable);
        retryTimedOutBoundary(boundaryFiber, retryLane);
      }
      function scheduleCallback(priorityLevel, callback) {
        return scheduleCallback$3(priorityLevel, callback);
      }
      function FiberNode(tag, pendingProps, key, mode) {
        this.tag = tag;
        this.key = key;
        this.sibling = this.child = this.return = this.stateNode = this.type = this.elementType = null;
        this.index = 0;
        this.refCleanup = this.ref = null;
        this.pendingProps = pendingProps;
        this.dependencies = this.memoizedState = this.updateQueue = this.memoizedProps = null;
        this.mode = mode;
        this.subtreeFlags = this.flags = 0;
        this.deletions = null;
        this.childLanes = this.lanes = 0;
        this.alternate = null;
      }
      function shouldConstruct(Component) {
        Component = Component.prototype;
        return !(!Component || !Component.isReactComponent);
      }
      function createWorkInProgress(current, pendingProps) {
        var workInProgress = current.alternate;
        workInProgress === null ? (workInProgress = createFiber(current.tag, pendingProps, current.key, current.mode), workInProgress.elementType = current.elementType, workInProgress.type = current.type, workInProgress.stateNode = current.stateNode, workInProgress.alternate = current, current.alternate = workInProgress) : (workInProgress.pendingProps = pendingProps, workInProgress.type = current.type, workInProgress.flags = 0, workInProgress.subtreeFlags = 0, workInProgress.deletions = null);
        workInProgress.flags = current.flags & 1206910976;
        workInProgress.childLanes = current.childLanes;
        workInProgress.lanes = current.lanes;
        workInProgress.child = current.child;
        workInProgress.memoizedProps = current.memoizedProps;
        workInProgress.memoizedState = current.memoizedState;
        workInProgress.updateQueue = current.updateQueue;
        pendingProps = current.dependencies;
        workInProgress.dependencies = pendingProps === null ? null : {
          lanes: pendingProps.lanes,
          firstContext: pendingProps.firstContext
        };
        workInProgress.sibling = current.sibling;
        workInProgress.index = current.index;
        workInProgress.ref = current.ref;
        workInProgress.refCleanup = current.refCleanup;
        return workInProgress;
      }
      function resetWorkInProgress(workInProgress, renderLanes) {
        workInProgress.flags &= 1206910978;
        var current = workInProgress.alternate;
        current === null ? (workInProgress.childLanes = 0, workInProgress.lanes = renderLanes, workInProgress.child = null, workInProgress.subtreeFlags = 0, workInProgress.memoizedProps = null, workInProgress.memoizedState = null, workInProgress.updateQueue = null, workInProgress.dependencies = null, workInProgress.stateNode = null) : (workInProgress.childLanes = current.childLanes, workInProgress.lanes = current.lanes, workInProgress.child = current.child, workInProgress.subtreeFlags = 0, workInProgress.deletions = null, workInProgress.memoizedProps = current.memoizedProps, workInProgress.memoizedState = current.memoizedState, workInProgress.updateQueue = current.updateQueue, workInProgress.type = current.type, renderLanes = current.dependencies, workInProgress.dependencies = renderLanes === null ? null : {
          lanes: renderLanes.lanes,
          firstContext: renderLanes.firstContext
        });
        return workInProgress;
      }
      function createFiberFromTypeAndProps(type, key, pendingProps, owner, mode, lanes) {
        var fiberTag = 0;
        owner = type;
        if (typeof owner === "function")
          shouldConstruct(owner) && (fiberTag = 1);
        else if (typeof owner === "string")
          fiberTag = supportsResources && supportsSingletons ? isHostHoistableType(type, pendingProps, contextStackCursor.current) ? 26 : isHostSingletonType(type) ? 27 : 5 : supportsResources ? isHostHoistableType(type, pendingProps, contextStackCursor.current) ? 26 : 5 : supportsSingletons ? isHostSingletonType(type) ? 27 : 5 : 5;
        else
          a:
            switch (owner) {
              case REACT_ACTIVITY_TYPE:
                return type = createFiber(31, pendingProps, key, mode), type.elementType = REACT_ACTIVITY_TYPE, type.lanes = lanes, type;
              case REACT_FRAGMENT_TYPE:
                return createFiberFromFragment(pendingProps.children, mode, lanes, key);
              case REACT_STRICT_MODE_TYPE:
                fiberTag = 8;
                mode |= 24;
                break;
              case REACT_PROFILER_TYPE:
                return type = createFiber(12, pendingProps, key, mode | 2), type.elementType = REACT_PROFILER_TYPE, type.lanes = lanes, type;
              case REACT_SUSPENSE_TYPE:
                return type = createFiber(13, pendingProps, key, mode), type.elementType = REACT_SUSPENSE_TYPE, type.lanes = lanes, type;
              case REACT_SUSPENSE_LIST_TYPE:
                return type = createFiber(19, pendingProps, key, mode), type.elementType = REACT_SUSPENSE_LIST_TYPE, type.lanes = lanes, type;
              case REACT_LEGACY_HIDDEN_TYPE:
              case REACT_VIEW_TRANSITION_TYPE:
                return type = mode | 32, type = createFiber(30, pendingProps, key, type), type.elementType = REACT_VIEW_TRANSITION_TYPE, type.lanes = lanes, type.stateNode = {
                  autoName: null,
                  paired: null,
                  clones: null,
                  ref: null
                }, type;
              default:
                if (typeof owner === "object" && owner !== null)
                  switch (owner.$$typeof) {
                    case REACT_CONTEXT_TYPE:
                      fiberTag = 10;
                      break a;
                    case REACT_CONSUMER_TYPE:
                      fiberTag = 9;
                      break a;
                    case REACT_FORWARD_REF_TYPE:
                      fiberTag = 11;
                      break a;
                    case REACT_MEMO_TYPE:
                      fiberTag = 14;
                      break a;
                    case REACT_LAZY_TYPE:
                      fiberTag = 16;
                      owner = null;
                      break a;
                  }
                fiberTag = 29;
                pendingProps = Error(formatProdErrorMessage(130, type === null ? "null" : typeof type, ""));
                owner = null;
            }
        key = createFiber(fiberTag, pendingProps, key, mode);
        key.elementType = type;
        key.type = owner;
        key.lanes = lanes;
        return key;
      }
      function createFiberFromFragment(elements, mode, lanes, key) {
        elements = createFiber(7, elements, key, mode);
        elements.lanes = lanes;
        return elements;
      }
      function createFiberFromText(content, mode, lanes) {
        content = createFiber(6, content, null, mode);
        content.lanes = lanes;
        return content;
      }
      function createFiberFromDehydratedFragment(dehydratedNode) {
        var fiber = createFiber(18, null, null, 0);
        fiber.stateNode = dehydratedNode;
        return fiber;
      }
      function createFiberFromPortal(portal, mode, lanes) {
        mode = createFiber(4, portal.children !== null ? portal.children : [], portal.key, mode);
        mode.lanes = lanes;
        mode.stateNode = {
          containerInfo: portal.containerInfo,
          pendingChildren: null,
          implementation: portal.implementation
        };
        return mode;
      }
      function FiberRootNode(containerInfo, tag, hydrate, identifierPrefix, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator, formState) {
        this.tag = 1;
        this.containerInfo = containerInfo;
        this.pingCache = this.current = this.pendingChildren = null;
        this.timeoutHandle = noTimeout;
        this.callbackNode = this.next = this.pendingContext = this.context = this.cancelPendingCommit = null;
        this.callbackPriority = 0;
        this.expirationTimes = createLaneMap(-1);
        this.entangledLanes = this.shellSuspendCounter = this.errorRecoveryDisabledLanes = this.expiredLanes = this.warmLanes = this.pingedLanes = this.suspendedLanes = this.pendingLanes = 0;
        this.entanglements = createLaneMap(0);
        this.hiddenUpdates = createLaneMap(null);
        this.identifierPrefix = identifierPrefix;
        this.onUncaughtError = onUncaughtError;
        this.onCaughtError = onCaughtError;
        this.onRecoverableError = onRecoverableError;
        this.pooledCache = null;
        this.pooledCacheLanes = 0;
        this.formState = formState;
        this.transitionTypes = null;
        this.incompleteTransitions = new Map;
      }
      function createFiberRoot(containerInfo, tag, hydrate, initialChildren, hydrationCallbacks, isStrictMode, identifierPrefix, formState, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator) {
        containerInfo = new FiberRootNode(containerInfo, tag, hydrate, identifierPrefix, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator, formState);
        tag = 1;
        isStrictMode === true && (tag |= 24);
        isStrictMode = createFiber(3, null, null, tag);
        containerInfo.current = isStrictMode;
        isStrictMode.stateNode = containerInfo;
        tag = createCache();
        tag.refCount++;
        containerInfo.pooledCache = tag;
        tag.refCount++;
        isStrictMode.memoizedState = {
          element: initialChildren,
          isDehydrated: hydrate,
          cache: tag
        };
        initializeUpdateQueue(isStrictMode);
        return containerInfo;
      }
      function getContextForSubtree(parentComponent) {
        if (!parentComponent)
          return emptyContextObject;
        parentComponent = emptyContextObject;
        return parentComponent;
      }
      function findHostInstance(component) {
        var fiber = component._reactInternals;
        if (fiber === undefined) {
          if (typeof component.render === "function")
            throw Error(formatProdErrorMessage(188));
          component = Object.keys(component).join(",");
          throw Error(formatProdErrorMessage(268, component));
        }
        component = findCurrentFiberUsingSlowPath(fiber);
        component = component !== null ? findCurrentHostFiberImpl(component) : null;
        return component === null ? null : getPublicInstance(component.stateNode);
      }
      function updateContainerImpl(rootFiber, lane, element, container, parentComponent, callback) {
        parentComponent = getContextForSubtree(parentComponent);
        container.context === null ? container.context = parentComponent : container.pendingContext = parentComponent;
        container = createUpdate(lane);
        container.payload = { element };
        callback = callback === undefined ? null : callback;
        callback !== null && (container.callback = callback);
        element = enqueueUpdate(rootFiber, container, lane);
        element !== null && (scheduleUpdateOnFiber(element, rootFiber, lane), entangleTransitions(element, rootFiber, lane));
      }
      function markRetryLaneImpl(fiber, retryLane) {
        fiber = fiber.memoizedState;
        if (fiber !== null && fiber.dehydrated !== null) {
          var a = fiber.retryLane;
          fiber.retryLane = a !== 0 && a < retryLane ? a : retryLane;
        }
      }
      function markRetryLaneIfNotHydrated(fiber, retryLane) {
        markRetryLaneImpl(fiber, retryLane);
        (fiber = fiber.alternate) && markRetryLaneImpl(fiber, retryLane);
      }
      var exports2 = {};
      var React = require_react(), Scheduler = require_scheduler(), assign = Object.assign, REACT_LEGACY_ELEMENT_TYPE = Symbol.for("react.element"), REACT_ELEMENT_TYPE = Symbol.for("react.transitional.element"), REACT_PORTAL_TYPE = Symbol.for("react.portal"), REACT_FRAGMENT_TYPE = Symbol.for("react.fragment"), REACT_STRICT_MODE_TYPE = Symbol.for("react.strict_mode"), REACT_PROFILER_TYPE = Symbol.for("react.profiler"), REACT_CONSUMER_TYPE = Symbol.for("react.consumer"), REACT_CONTEXT_TYPE = Symbol.for("react.context"), REACT_FORWARD_REF_TYPE = Symbol.for("react.forward_ref"), REACT_SUSPENSE_TYPE = Symbol.for("react.suspense"), REACT_SUSPENSE_LIST_TYPE = Symbol.for("react.suspense_list"), REACT_MEMO_TYPE = Symbol.for("react.memo"), REACT_LAZY_TYPE = Symbol.for("react.lazy");
      Symbol.for("react.scope");
      var REACT_ACTIVITY_TYPE = Symbol.for("react.activity"), REACT_LEGACY_HIDDEN_TYPE = Symbol.for("react.legacy_hidden");
      Symbol.for("react.tracing_marker");
      var REACT_MEMO_CACHE_SENTINEL = Symbol.for("react.memo_cache_sentinel"), REACT_VIEW_TRANSITION_TYPE = Symbol.for("react.view_transition"), REACT_RECOVERABLE_TYPE = Symbol.for("react.recoverable"), MAYBE_ITERATOR_SYMBOL = Symbol.iterator, REACT_OPTIMISTIC_KEY = Symbol.for("react.optimistic_key"), REACT_CLIENT_REFERENCE = Symbol.for("react.client.reference"), isArrayImpl = Array.isArray, ReactSharedInternals = React.__CLIENT_INTERNALS_DO_NOT_USE_OR_WARN_USERS_THEY_CANNOT_UPGRADE, { rendererVersion, rendererPackageName, extraDevToolsConfig, getPublicInstance, getRootHostContext, getChildHostContext, prepareForCommit, resetAfterCommit, createInstance } = $$$config;
      $$$config.cloneMutableInstance;
      var { appendInitialChild, finalizeInitialChildren, shouldSetTextContent, createTextInstance } = $$$config;
      $$$config.cloneMutableTextInstance;
      var { scheduleTimeout, cancelTimeout, noTimeout, isPrimaryRenderer } = $$$config;
      $$$config.warnsIfNotActing;
      var { supportsMutation, supportsPersistence, supportsHydration, getInstanceFromNode } = $$$config;
      $$$config.beforeActiveInstanceBlur;
      var preparePortalMount = $$$config.preparePortalMount;
      $$$config.prepareScopeUpdate;
      $$$config.getInstanceFromScope;
      var { setCurrentUpdatePriority, getCurrentUpdatePriority, resolveUpdatePriority } = $$$config;
      $$$config.trackSchedulerEvent;
      $$$config.resolveEventType;
      $$$config.resolveEventTimeStamp;
      var { shouldAttemptEagerTransition, detachDeletedInstance } = $$$config;
      $$$config.requestPostPaintCallback;
      var { maySuspendCommit, maySuspendCommitOnUpdate, maySuspendCommitInSyncRender, preloadInstance, startSuspendingCommit, suspendInstance, suspendOnActiveViewTransition, waitForCommitToBeReady } = $$$config;
      $$$config.getSuspendedCommitReason;
      var { NotPendingTransition, HostTransitionContext, resetFormInstance } = $$$config;
      $$$config.bindToConsole;
      var { supportsMicrotasks, scheduleMicrotask, supportsTestSelectors, findFiberRoot, getBoundingRect, getTextContent, isHiddenSubtree, matchAccessibilityRole, setFocusIfFocusable, setupIntersectionObserver, appendChild, appendChildToContainer, commitTextUpdate, commitMount, commitUpdate, insertBefore, insertInContainerBefore, removeChild, removeChildFromContainer, resetTextContent, hideInstance, hideTextInstance, unhideInstance, unhideTextInstance, applyViewTransitionName, restoreViewTransitionName, cancelViewTransitionName, cancelRootViewTransitionName, restoreRootViewTransitionName } = $$$config;
      $$$config.cloneRootViewTransitionContainer;
      $$$config.removeRootViewTransitionClone;
      var { measureInstance, measureClonedInstance, wasInstanceInViewport, hasInstanceChanged, hasInstanceAffectedParent, startViewTransition } = $$$config;
      $$$config.startGestureTransition;
      var { stopViewTransition, addViewTransitionFinishedListener } = $$$config;
      $$$config.getCurrentGestureOffset;
      var { createViewTransitionInstance, clearContainer, createFragmentInstance, updateFragmentInstanceFiber, commitNewChildToFragmentInstance, deleteChildFromFragmentInstance, cloneInstance, createContainerChildSet, appendChildToContainerChildSet, finalizeContainerChildren, replaceContainerChildren, cloneHiddenInstance, cloneHiddenTextInstance, isSuspenseInstancePending, isSuspenseInstanceFallback, getSuspenseInstanceFallbackErrorDetails, registerSuspenseInstanceRetry, canHydrateFormStateMarker, isFormStateMarkerMatching, getNextHydratableSibling, getNextHydratableSiblingAfterSingleton, getFirstHydratableChild, getFirstHydratableChildWithinContainer, getFirstHydratableChildWithinActivityInstance, getFirstHydratableChildWithinSuspenseInstance, getFirstHydratableChildWithinSingleton, canHydrateInstance, canHydrateTextInstance, canHydrateActivityInstance, canHydrateSuspenseInstance, hydrateInstance, hydrateTextInstance, hydrateActivityInstance, hydrateSuspenseInstance, getNextHydratableInstanceAfterActivityInstance, getNextHydratableInstanceAfterSuspenseInstance, commitHydratedInstance, commitHydratedContainer, commitHydratedActivityInstance, commitHydratedSuspenseInstance, finalizeHydratedChildren, flushHydrationEvents } = $$$config;
      $$$config.clearActivityBoundary;
      var clearSuspenseBoundary = $$$config.clearSuspenseBoundary;
      $$$config.clearActivityBoundaryFromContainer;
      var { clearSuspenseBoundaryFromContainer, hideDehydratedBoundary, unhideDehydratedBoundary, shouldDeleteUnhydratedTailInstances } = $$$config;
      $$$config.diffHydratedPropsForDevWarnings;
      $$$config.diffHydratedTextForDevWarnings;
      $$$config.describeHydratableInstanceForDevWarnings;
      var { validateHydratableInstance, validateHydratableTextInstance, supportsResources, isHostHoistableType, getHoistableRoot, getResource, acquireResource, releaseResource, hydrateHoistable, mountHoistable, unmountHoistable, createHoistableInstance, prepareToCommitHoistables, mayResourceSuspendCommit, preloadResource, suspendResource, supportsSingletons, resolveSingletonInstance, acquireSingletonInstance, releaseSingletonInstance, isHostSingletonType, isSingletonScope } = $$$config, valueStack = [], index$jscomp$0 = -1, emptyContextObject = {}, clz32 = Math.clz32 ? Math.clz32 : clz32Fallback, { log: log$1, LN2 } = Math, nextTransitionUpdateLane = 256, nextTransitionDeferredLane = 262144, nextRetryLane = 4194304, { unstable_scheduleCallback: scheduleCallback$3, unstable_cancelCallback: cancelCallback$1, unstable_shouldYield: shouldYield, unstable_requestPaint: requestPaint, unstable_now: now, unstable_ImmediatePriority: ImmediatePriority, unstable_UserBlockingPriority: UserBlockingPriority, unstable_NormalPriority: NormalPriority$1, unstable_IdlePriority: IdlePriority, log, unstable_setDisableYieldValue } = Scheduler, rendererID = null, injectedHook = null, globalClientIdCounter$1 = 0, objectIs = typeof Object.is === "function" ? Object.is : is, reportGlobalError = typeof reportError === "function" ? reportError : function(error) {
        if (typeof window === "object" && typeof window.ErrorEvent === "function") {
          var event = new window.ErrorEvent("error", {
            bubbles: true,
            cancelable: true,
            message: typeof error === "object" && error !== null && typeof error.message === "string" ? String(error.message) : String(error),
            error
          });
          if (!window.dispatchEvent(event))
            return;
        } else if (typeof process === "object" && typeof process.emit === "function") {
          process.emit("uncaughtException", error);
          return;
        }
        console.error(error);
      }, hasOwnProperty = Object.prototype.hasOwnProperty, prefix, suffix, reentry = false, CapturedStacks = new WeakMap, forkStack = [], forkStackIndex = 0, treeForkProvider = null, treeForkCount = 0, idStack = [], idStackIndex = 0, treeContextProvider = null, treeContextId = 1, treeContextOverflow = "", contextStackCursor = createCursor(null), contextFiberStackCursor = createCursor(null), rootInstanceStackCursor = createCursor(null), hostTransitionProviderCursor = createCursor(null), hydrationParentFiber = null, nextHydratableInstance = null, isHydrating = false, hydrationErrors = null, rootOrSingletonContext = false, HydrationMismatchException = Error(formatProdErrorMessage(519)), valueCursor = createCursor(null), currentlyRenderingFiber$1 = null, lastContextDependency = null, AbortControllerLocal = typeof AbortController !== "undefined" ? AbortController : function() {
        var listeners = [], signal = this.signal = {
          aborted: false,
          addEventListener: function(type, listener) {
            listeners.push(listener);
          }
        };
        this.abort = function() {
          signal.aborted = true;
          listeners.forEach(function(listener) {
            return listener();
          });
        };
      }, { unstable_scheduleCallback: scheduleCallback$2, unstable_NormalPriority: NormalPriority } = Scheduler, CacheContext = {
        $$typeof: REACT_CONTEXT_TYPE,
        Consumer: null,
        Provider: null,
        _currentValue: null,
        _currentValue2: null,
        _threadCount: 0
      }, entangledTransitionTypes = null, firstScheduledRoot = null, lastScheduledRoot = null, didScheduleMicrotask = false, mightHavePendingSyncWork = false, isFlushingWork = false, currentEventTransitionLane = 0, currentEntangledListeners = null, currentEntangledPendingCount = 0, currentEntangledLane = 0, currentEntangledActionThenable = null, prevOnStartTransitionFinish = ReactSharedInternals.S;
      ReactSharedInternals.S = function(transition, returnValue) {
        globalMostRecentTransitionTime = now();
        typeof returnValue === "object" && returnValue !== null && typeof returnValue.then === "function" && entangleAsyncAction(transition, returnValue);
        if (entangledTransitionTypes !== null)
          for (var root = firstScheduledRoot;root !== null; )
            queueTransitionTypes(root, entangledTransitionTypes), root = root.next;
        root = transition.types;
        if (root !== null) {
          for (var root$19 = firstScheduledRoot;root$19 !== null; )
            queueTransitionTypes(root$19, root), root$19 = root$19.next;
          if (currentEntangledLane !== 0) {
            root$19 = entangledTransitionTypes;
            root$19 === null && (root$19 = entangledTransitionTypes = []);
            for (var i = 0;i < root.length; i++) {
              var transitionType = root[i];
              root$19.indexOf(transitionType) === -1 && root$19.push(transitionType);
            }
          }
        }
        prevOnStartTransitionFinish !== null && prevOnStartTransitionFinish(transition, returnValue);
      };
      var resumedCache = createCursor(null), SuspenseException = Error(formatProdErrorMessage(460)), SuspenseyCommitException = Error(formatProdErrorMessage(474)), SuspenseActionException = Error(formatProdErrorMessage(542)), noopSuspenseyCommitThenable = { then: function() {} }, suspendedThenable = null, thenableState$1 = null, thenableIndexCounter$1 = 0, reconcileChildFibers = createChildReconciler(true), mountChildFibers = createChildReconciler(false), concurrentQueues = [], concurrentQueuesIndex = 0, concurrentlyUpdatedLanes = 0, hasForceUpdate = false, didReadFromEntangledAsyncAction = false, currentTreeHiddenStackCursor = createCursor(null), prevEntangledRenderLanesCursor = createCursor(0), suspenseHandlerStackCursor = createCursor(null), shellBoundary = null, suspenseStackCursor = createCursor(0), renderLanes = 0, currentlyRenderingFiber = null, currentHook = null, workInProgressHook = null, didScheduleRenderPhaseUpdate = false, didScheduleRenderPhaseUpdateDuringThisPass = false, shouldDoubleInvokeUserFnsInHooksDEV = false, localIdCounter = 0, thenableIndexCounter = 0, thenableState = null, globalClientIdCounter = 0, ContextOnlyDispatcher = {
        readContext,
        use,
        useCallback: throwInvalidHookError,
        useContext: throwInvalidHookError,
        useEffect: throwInvalidHookError,
        useImperativeHandle: throwInvalidHookError,
        useLayoutEffect: throwInvalidHookError,
        useInsertionEffect: throwInvalidHookError,
        useMemo: throwInvalidHookError,
        useReducer: throwInvalidHookError,
        useRef: throwInvalidHookError,
        useState: throwInvalidHookError,
        useDebugValue: throwInvalidHookError,
        useDeferredValue: throwInvalidHookError,
        useTransition: throwInvalidHookError,
        useSyncExternalStore: throwInvalidHookError,
        useId: throwInvalidHookError,
        useHostTransitionStatus: throwInvalidHookError,
        useFormState: throwInvalidHookError,
        useActionState: throwInvalidHookError,
        useOptimistic: throwInvalidHookError,
        useMemoCache: throwInvalidHookError,
        useCacheRefresh: throwInvalidHookError,
        useEffectEvent: throwInvalidHookError
      }, HooksDispatcherOnMount = {
        readContext,
        use,
        useCallback: function(callback, deps) {
          mountWorkInProgressHook().memoizedState = [
            callback,
            deps === undefined ? null : deps
          ];
          return callback;
        },
        useContext: readContext,
        useEffect: mountEffect,
        useImperativeHandle: function(ref, create, deps) {
          deps = deps !== null && deps !== undefined ? deps.concat([ref]) : null;
          mountEffectImpl(4194308, 4, imperativeHandleEffect.bind(null, create, ref), deps);
        },
        useLayoutEffect: function(create, deps) {
          return mountEffectImpl(4194308, 4, create, deps);
        },
        useInsertionEffect: function(create, deps) {
          mountEffectImpl(4, 2, create, deps);
        },
        useMemo: function(nextCreate, deps) {
          var hook = mountWorkInProgressHook();
          deps = deps === undefined ? null : deps;
          var nextValue = nextCreate();
          if (shouldDoubleInvokeUserFnsInHooksDEV) {
            setIsStrictModeForDevtools(true);
            try {
              nextCreate();
            } finally {
              setIsStrictModeForDevtools(false);
            }
          }
          hook.memoizedState = [nextValue, deps];
          return nextValue;
        },
        useReducer: function(reducer, initialArg, init) {
          var hook = mountWorkInProgressHook();
          if (init !== undefined) {
            var initialState = init(initialArg);
            if (shouldDoubleInvokeUserFnsInHooksDEV) {
              setIsStrictModeForDevtools(true);
              try {
                init(initialArg);
              } finally {
                setIsStrictModeForDevtools(false);
              }
            }
          } else
            initialState = initialArg;
          hook.memoizedState = hook.baseState = initialState;
          reducer = {
            pending: null,
            lanes: 0,
            dispatch: null,
            lastRenderedReducer: reducer,
            lastRenderedState: initialState
          };
          hook.queue = reducer;
          reducer = reducer.dispatch = dispatchReducerAction.bind(null, currentlyRenderingFiber, reducer);
          return [hook.memoizedState, reducer];
        },
        useRef: function(initialValue) {
          var hook = mountWorkInProgressHook();
          initialValue = { current: initialValue };
          return hook.memoizedState = initialValue;
        },
        useState: function(initialState) {
          initialState = mountStateImpl(initialState);
          var queue = initialState.queue, dispatch = dispatchSetState.bind(null, currentlyRenderingFiber, queue);
          queue.dispatch = dispatch;
          return [initialState.memoizedState, dispatch];
        },
        useDebugValue: mountDebugValue,
        useDeferredValue: function(value, initialValue) {
          var hook = mountWorkInProgressHook();
          return mountDeferredValueImpl(hook, value, initialValue);
        },
        useTransition: function() {
          var stateHook = mountStateImpl(false);
          stateHook = startTransition.bind(null, currentlyRenderingFiber, stateHook.queue, true, false);
          mountWorkInProgressHook().memoizedState = stateHook;
          return [false, stateHook];
        },
        useSyncExternalStore: function(subscribe, getSnapshot, getServerSnapshot) {
          var fiber = currentlyRenderingFiber, hook = mountWorkInProgressHook();
          if (isHydrating) {
            if (getServerSnapshot === undefined)
              throw Error(formatProdErrorMessage(407));
            getServerSnapshot = getServerSnapshot();
          } else {
            getServerSnapshot = getSnapshot();
            if (workInProgressRoot === null)
              throw Error(formatProdErrorMessage(349));
            (workInProgressRootRenderLanes & 127) !== 0 || pushStoreConsistencyCheck(fiber, getSnapshot, getServerSnapshot);
          }
          hook.memoizedState = getServerSnapshot;
          var inst = { value: getServerSnapshot, getSnapshot };
          hook.queue = inst;
          mountEffect(subscribeToStore.bind(null, fiber, inst, subscribe), [
            subscribe
          ]);
          fiber.flags |= 2048;
          pushSimpleEffect(9, { destroy: undefined }, updateStoreInstance.bind(null, fiber, inst, getServerSnapshot, getSnapshot), null);
          return getServerSnapshot;
        },
        useId: function() {
          var hook = mountWorkInProgressHook(), identifierPrefix = workInProgressRoot.identifierPrefix;
          if (isHydrating) {
            var JSCompiler_inline_result = treeContextOverflow;
            var idWithLeadingBit = treeContextId;
            JSCompiler_inline_result = (idWithLeadingBit & ~(1 << 32 - clz32(idWithLeadingBit) - 1)).toString(32) + JSCompiler_inline_result;
            identifierPrefix = "_" + identifierPrefix + "R_" + JSCompiler_inline_result;
            JSCompiler_inline_result = localIdCounter++;
            0 < JSCompiler_inline_result && (identifierPrefix += "H" + JSCompiler_inline_result.toString(32));
            identifierPrefix += "_";
          } else
            JSCompiler_inline_result = globalClientIdCounter++, identifierPrefix = "_" + identifierPrefix + "r_" + JSCompiler_inline_result.toString(32) + "_";
          return hook.memoizedState = identifierPrefix;
        },
        useHostTransitionStatus,
        useFormState: mountActionState,
        useActionState: mountActionState,
        useOptimistic: function(passthrough) {
          var hook = mountWorkInProgressHook();
          hook.memoizedState = hook.baseState = passthrough;
          var queue = {
            pending: null,
            lanes: 0,
            dispatch: null,
            lastRenderedReducer: null,
            lastRenderedState: null
          };
          hook.queue = queue;
          hook = dispatchOptimisticSetState.bind(null, currentlyRenderingFiber, true, queue);
          queue.dispatch = hook;
          return [passthrough, hook];
        },
        useMemoCache,
        useCacheRefresh: function() {
          return mountWorkInProgressHook().memoizedState = refreshCache.bind(null, currentlyRenderingFiber);
        },
        useEffectEvent: function(callback) {
          var hook = mountWorkInProgressHook(), ref = { impl: callback };
          hook.memoizedState = ref;
          return function() {
            if ((executionContext & 2) !== 0)
              throw Error(formatProdErrorMessage(440));
            return ref.impl.apply(undefined, arguments);
          };
        }
      }, HooksDispatcherOnUpdate = {
        readContext,
        use,
        useCallback: updateCallback,
        useContext: readContext,
        useEffect: updateEffect,
        useImperativeHandle: updateImperativeHandle,
        useInsertionEffect: updateInsertionEffect,
        useLayoutEffect: updateLayoutEffect,
        useMemo: updateMemo,
        useReducer: updateReducer,
        useRef: updateRef,
        useState: function() {
          return updateReducer(basicStateReducer);
        },
        useDebugValue: mountDebugValue,
        useDeferredValue: function(value, initialValue) {
          var hook = updateWorkInProgressHook();
          return updateDeferredValueImpl(hook, currentHook.memoizedState, value, initialValue);
        },
        useTransition: function() {
          var booleanOrThenable = updateReducer(basicStateReducer)[0], start = updateWorkInProgressHook().memoizedState;
          return [
            typeof booleanOrThenable === "boolean" ? booleanOrThenable : useThenable(booleanOrThenable),
            start
          ];
        },
        useSyncExternalStore: updateSyncExternalStore,
        useId: updateId,
        useHostTransitionStatus,
        useFormState: updateActionState,
        useActionState: updateActionState,
        useOptimistic: function(passthrough, reducer) {
          var hook = updateWorkInProgressHook();
          return updateOptimisticImpl(hook, currentHook, passthrough, reducer);
        },
        useMemoCache,
        useCacheRefresh: updateRefresh,
        useEffectEvent: updateEvent
      }, HooksDispatcherOnRerender = {
        readContext,
        use,
        useCallback: updateCallback,
        useContext: readContext,
        useEffect: updateEffect,
        useImperativeHandle: updateImperativeHandle,
        useInsertionEffect: updateInsertionEffect,
        useLayoutEffect: updateLayoutEffect,
        useMemo: updateMemo,
        useReducer: rerenderReducer,
        useRef: updateRef,
        useState: function() {
          return rerenderReducer(basicStateReducer);
        },
        useDebugValue: mountDebugValue,
        useDeferredValue: function(value, initialValue) {
          var hook = updateWorkInProgressHook();
          return currentHook === null ? mountDeferredValueImpl(hook, value, initialValue) : updateDeferredValueImpl(hook, currentHook.memoizedState, value, initialValue);
        },
        useTransition: function() {
          var booleanOrThenable = rerenderReducer(basicStateReducer)[0], start = updateWorkInProgressHook().memoizedState;
          return [
            typeof booleanOrThenable === "boolean" ? booleanOrThenable : useThenable(booleanOrThenable),
            start
          ];
        },
        useSyncExternalStore: updateSyncExternalStore,
        useId: updateId,
        useHostTransitionStatus,
        useFormState: rerenderActionState,
        useActionState: rerenderActionState,
        useOptimistic: function(passthrough, reducer) {
          var hook = updateWorkInProgressHook();
          if (currentHook !== null)
            return updateOptimisticImpl(hook, currentHook, passthrough, reducer);
          hook.baseState = passthrough;
          return [passthrough, hook.queue.dispatch];
        },
        useMemoCache,
        useCacheRefresh: updateRefresh,
        useEffectEvent: updateEvent
      }, classComponentUpdater = {
        enqueueSetState: function(inst, payload, callback) {
          inst = inst._reactInternals;
          var lane = requestUpdateLane(), update = createUpdate(lane);
          update.payload = payload;
          callback !== undefined && callback !== null && (update.callback = callback);
          payload = enqueueUpdate(inst, update, lane);
          payload !== null && (scheduleUpdateOnFiber(payload, inst, lane), entangleTransitions(payload, inst, lane));
        },
        enqueueReplaceState: function(inst, payload, callback) {
          inst = inst._reactInternals;
          var lane = requestUpdateLane(), update = createUpdate(lane);
          update.tag = 1;
          update.payload = payload;
          callback !== undefined && callback !== null && (update.callback = callback);
          payload = enqueueUpdate(inst, update, lane);
          payload !== null && (scheduleUpdateOnFiber(payload, inst, lane), entangleTransitions(payload, inst, lane));
        },
        enqueueForceUpdate: function(inst, callback) {
          inst = inst._reactInternals;
          var lane = requestUpdateLane(), update = createUpdate(lane);
          update.tag = 2;
          callback !== undefined && callback !== null && (update.callback = callback);
          callback = enqueueUpdate(inst, update, lane);
          callback !== null && (scheduleUpdateOnFiber(callback, inst, lane), entangleTransitions(callback, inst, lane));
        }
      }, SelectiveHydrationException = Error(formatProdErrorMessage(461)), didReceiveUpdate = false, SUSPENDED_MARKER = {
        dehydrated: null,
        treeContext: null,
        retryLane: 0,
        hydrationErrors: null
      }, viewTransitionMutationContext = false, shouldStartViewTransition = false, appearingViewTransitions = null, viewTransitionCancelableChildren = null, viewTransitionHostInstanceIdx = 0, offscreenSubtreeIsHidden = false, offscreenSubtreeWasHidden = false, offscreenDirectParentIsHidden = false, needsFormReset = false, PossiblyWeakSet = typeof WeakSet === "function" ? WeakSet : Set, nextEffect = null, viewTransitionContextChanged = false, inUpdateViewTransition = false, rootViewTransitionAffected = false, rootViewTransitionNameCanceled = false, hostParent = null, hostParentIsContainer = false, currentHoistableRoot = null, suspenseyCommitFlag = 8192, DefaultAsyncDispatcher = {
        getCacheForType: function(resourceType) {
          var cache = readContext(CacheContext), cacheForType = cache.data.get(resourceType);
          cacheForType === undefined && (cacheForType = resourceType(), cache.data.set(resourceType, cacheForType));
          return cacheForType;
        },
        cacheSignal: function() {
          return readContext(CacheContext).controller.signal;
        }
      }, COMPONENT_TYPE = 0, HAS_PSEUDO_CLASS_TYPE = 1, ROLE_TYPE = 2, TEST_NAME_TYPE = 3, TEXT_TYPE = 4;
      if (typeof Symbol === "function" && Symbol.for) {
        var symbolFor = Symbol.for;
        COMPONENT_TYPE = symbolFor("selector.component");
        HAS_PSEUDO_CLASS_TYPE = symbolFor("selector.has_pseudo_class");
        ROLE_TYPE = symbolFor("selector.role");
        TEST_NAME_TYPE = symbolFor("selector.test_id");
        TEXT_TYPE = symbolFor("selector.text");
      }
      var PossiblyWeakMap = typeof WeakMap === "function" ? WeakMap : Map, executionContext = 0, workInProgressRoot = null, workInProgress = null, workInProgressRootRenderLanes = 0, workInProgressSuspendedReason = 0, workInProgressThrownValue = null, workInProgressRootDidSkipSuspendedSiblings = false, workInProgressRootIsPrerendering = false, workInProgressRootDidAttachPingListener = false, entangledRenderLanes = 0, workInProgressRootExitStatus = 0, workInProgressRootSkippedLanes = 0, workInProgressRootInterleavedUpdatedLanes = 0, workInProgressRootPingedLanes = 0, workInProgressDeferredLane = 0, workInProgressSuspendedRetryLanes = 0, workInProgressRootConcurrentErrors = null, workInProgressRootRecoverableErrors = null, workInProgressRootDidIncludeRecursiveRenderUpdate = false, globalMostRecentFallbackTime = 0, globalMostRecentTransitionTime = 0, workInProgressRootRenderTargetTime = Infinity, workInProgressTransitions = null, legacyErrorBoundariesThatAlreadyFailed = null, pendingEffectsStatus = 0, pendingEffectsRoot = null, pendingFinishedWork = null, pendingEffectsLanes = 0, pendingEffectsRemainingLanes = 0, pendingPassiveTransitions = null, pendingRecoverableErrors = null, pendingViewTransition = null, pendingViewTransitionEvents = null, pendingTransitionTypes = null, nestedUpdateCount = 0, rootWithNestedUpdates = null;
      exports2.attemptContinuousHydration = function(fiber) {
        if (fiber.tag === 13 || fiber.tag === 31) {
          var root = enqueueConcurrentRenderForLane(fiber, 67108864);
          root !== null && scheduleUpdateOnFiber(root, fiber, 67108864);
          markRetryLaneIfNotHydrated(fiber, 67108864);
        }
      };
      exports2.attemptHydrationAtCurrentPriority = function(fiber) {
        if (fiber.tag === 13 || fiber.tag === 31) {
          var lane = requestUpdateLane();
          lane = getBumpedLaneForHydrationByLane(lane);
          var root = enqueueConcurrentRenderForLane(fiber, lane);
          root !== null && scheduleUpdateOnFiber(root, fiber, lane);
          markRetryLaneIfNotHydrated(fiber, lane);
        }
      };
      exports2.attemptSynchronousHydration = function(fiber) {
        switch (fiber.tag) {
          case 3:
            fiber = fiber.stateNode;
            if (fiber.current.memoizedState.isDehydrated) {
              var lanes = getHighestPriorityLanes(fiber.pendingLanes);
              if (lanes !== 0) {
                fiber.pendingLanes |= 2;
                for (fiber.entangledLanes |= 2;lanes; ) {
                  var lane = 1 << 31 - clz32(lanes);
                  fiber.entanglements[1] |= lane;
                  lanes &= ~lane;
                }
                ensureRootIsScheduled(fiber);
                (executionContext & 6) === 0 && (workInProgressRootRenderTargetTime = now() + 500, flushSyncWorkAcrossRoots_impl(0, false));
              }
            }
            break;
          case 31:
          case 13:
            lanes = enqueueConcurrentRenderForLane(fiber, 2), lanes !== null && scheduleUpdateOnFiber(lanes, fiber, 2), flushSyncWork(), markRetryLaneIfNotHydrated(fiber, 2);
        }
      };
      exports2.batchedUpdates = function(fn, a) {
        return fn(a);
      };
      exports2.createComponentSelector = function(component) {
        return { $$typeof: COMPONENT_TYPE, value: component };
      };
      exports2.createContainer = function(containerInfo, tag, hydrationCallbacks, isStrictMode, concurrentUpdatesByDefaultOverride, identifierPrefix, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator) {
        return createFiberRoot(containerInfo, tag, false, null, hydrationCallbacks, isStrictMode, identifierPrefix, null, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator);
      };
      exports2.createHasPseudoClassSelector = function(selectors) {
        return { $$typeof: HAS_PSEUDO_CLASS_TYPE, value: selectors };
      };
      exports2.createHydrationContainer = function(initialChildren, callback, containerInfo, tag, hydrationCallbacks, isStrictMode, concurrentUpdatesByDefaultOverride, identifierPrefix, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator, transitionCallbacks, formState) {
        initialChildren = createFiberRoot(containerInfo, tag, true, initialChildren, hydrationCallbacks, isStrictMode, identifierPrefix, formState, onUncaughtError, onCaughtError, onRecoverableError, onDefaultTransitionIndicator);
        initialChildren.context = getContextForSubtree(null);
        containerInfo = initialChildren.current;
        tag = requestUpdateLane();
        tag = getBumpedLaneForHydrationByLane(tag);
        hydrationCallbacks = createUpdate(tag);
        hydrationCallbacks.callback = callback !== undefined && callback !== null ? callback : null;
        enqueueUpdate(containerInfo, hydrationCallbacks, tag);
        callback = tag;
        initialChildren.current.lanes = callback;
        markRootUpdated$1(initialChildren, callback);
        ensureRootIsScheduled(initialChildren);
        return initialChildren;
      };
      exports2.createPortal = function(children, containerInfo, implementation) {
        var key = 3 < arguments.length && arguments[3] !== undefined ? arguments[3] : null;
        return {
          $$typeof: REACT_PORTAL_TYPE,
          key: key == null ? null : key === REACT_OPTIMISTIC_KEY ? REACT_OPTIMISTIC_KEY : "" + key,
          children,
          containerInfo,
          implementation
        };
      };
      exports2.createRoleSelector = function(role) {
        return { $$typeof: ROLE_TYPE, value: role };
      };
      exports2.createTestNameSelector = function(id) {
        return { $$typeof: TEST_NAME_TYPE, value: id };
      };
      exports2.createTextSelector = function(text) {
        return { $$typeof: TEXT_TYPE, value: text };
      };
      exports2.defaultOnCaughtError = function(error) {
        console.error(error);
      };
      exports2.defaultOnRecoverableError = function(error) {
        reportGlobalError(error);
      };
      exports2.defaultOnUncaughtError = function(error) {
        reportGlobalError(error);
      };
      exports2.deferredUpdates = function(fn) {
        var prevTransition = ReactSharedInternals.T, previousPriority = getCurrentUpdatePriority();
        try {
          return setCurrentUpdatePriority(32), ReactSharedInternals.T = null, fn();
        } finally {
          setCurrentUpdatePriority(previousPriority), ReactSharedInternals.T = prevTransition;
        }
      };
      exports2.discreteUpdates = function(fn, a, b, c, d) {
        var prevTransition = ReactSharedInternals.T, previousPriority = getCurrentUpdatePriority();
        try {
          return setCurrentUpdatePriority(2), ReactSharedInternals.T = null, fn(a, b, c, d);
        } finally {
          setCurrentUpdatePriority(previousPriority), ReactSharedInternals.T = prevTransition, executionContext === 0 && (workInProgressRootRenderTargetTime = now() + 500);
        }
      };
      exports2.findAllNodes = findAllNodes;
      exports2.findBoundingRects = function(hostRoot, selectors) {
        if (!supportsTestSelectors)
          throw Error(formatProdErrorMessage(363));
        selectors = findAllNodes(hostRoot, selectors);
        hostRoot = [];
        for (var i = 0;i < selectors.length; i++)
          hostRoot.push(getBoundingRect(selectors[i]));
        for (selectors = hostRoot.length - 1;0 < selectors; selectors--) {
          i = hostRoot[selectors];
          for (var targetLeft = i.x, targetRight = targetLeft + i.width, targetTop = i.y, targetBottom = targetTop + i.height, j = selectors - 1;0 <= j; j--)
            if (selectors !== j) {
              var otherRect = hostRoot[j], otherLeft = otherRect.x, otherRight = otherLeft + otherRect.width, otherTop = otherRect.y, otherBottom = otherTop + otherRect.height;
              if (targetLeft >= otherLeft && targetTop >= otherTop && targetRight <= otherRight && targetBottom <= otherBottom) {
                hostRoot.splice(selectors, 1);
                break;
              } else if (!(targetLeft !== otherLeft || i.width !== otherRect.width || otherBottom < targetTop || otherTop > targetBottom)) {
                otherTop > targetTop && (otherRect.height += otherTop - targetTop, otherRect.y = targetTop);
                otherBottom < targetBottom && (otherRect.height = targetBottom - otherTop);
                hostRoot.splice(selectors, 1);
                break;
              } else if (!(targetTop !== otherTop || i.height !== otherRect.height || otherRight < targetLeft || otherLeft > targetRight)) {
                otherLeft > targetLeft && (otherRect.width += otherLeft - targetLeft, otherRect.x = targetLeft);
                otherRight < targetRight && (otherRect.width = targetRight - otherLeft);
                hostRoot.splice(selectors, 1);
                break;
              }
            }
        }
        return hostRoot;
      };
      exports2.findHostInstance = findHostInstance;
      exports2.findHostInstanceWithNoPortals = function(fiber) {
        fiber = findCurrentFiberUsingSlowPath(fiber);
        fiber = fiber !== null ? findCurrentHostFiberWithNoPortalsImpl(fiber) : null;
        return fiber === null ? null : getPublicInstance(fiber.stateNode);
      };
      exports2.findHostInstanceWithWarning = function(component) {
        return findHostInstance(component);
      };
      exports2.flushPassiveEffects = flushPendingEffects;
      exports2.flushSyncFromReconciler = function(fn) {
        var prevExecutionContext = executionContext;
        executionContext |= 1;
        var prevTransition = ReactSharedInternals.T, previousPriority = getCurrentUpdatePriority();
        try {
          if (setCurrentUpdatePriority(2), ReactSharedInternals.T = null, fn)
            return fn();
        } finally {
          setCurrentUpdatePriority(previousPriority), ReactSharedInternals.T = prevTransition, executionContext = prevExecutionContext, (executionContext & 6) === 0 && flushSyncWorkAcrossRoots_impl(0, false);
        }
      };
      exports2.flushSyncWork = flushSyncWork;
      exports2.focusWithin = function(hostRoot, selectors) {
        if (!supportsTestSelectors)
          throw Error(formatProdErrorMessage(363));
        hostRoot = findFiberRootForHostRoot(hostRoot);
        selectors = findPaths(hostRoot, selectors);
        selectors = Array.from(selectors);
        for (hostRoot = 0;hostRoot < selectors.length; ) {
          var fiber = selectors[hostRoot++], tag = fiber.tag;
          if (!isHiddenSubtree(fiber)) {
            if ((tag === 5 || tag === 26 || tag === 27) && setFocusIfFocusable(fiber.stateNode))
              return true;
            for (fiber = fiber.child;fiber !== null; )
              selectors.push(fiber), fiber = fiber.sibling;
          }
        }
        return false;
      };
      exports2.getFindAllNodesFailureDescription = function(hostRoot, selectors) {
        if (!supportsTestSelectors)
          throw Error(formatProdErrorMessage(363));
        var maxSelectorIndex = 0, matchedNames = [];
        hostRoot = [findFiberRootForHostRoot(hostRoot), 0];
        for (var index = 0;index < hostRoot.length; ) {
          var fiber = hostRoot[index++], tag = fiber.tag, selectorIndex = hostRoot[index++], selector = selectors[selectorIndex];
          if (tag !== 5 && tag !== 26 && tag !== 27 || !isHiddenSubtree(fiber)) {
            if (matchSelector(fiber, selector) && (matchedNames.push(selectorToString(selector)), selectorIndex++, selectorIndex > maxSelectorIndex && (maxSelectorIndex = selectorIndex)), selectorIndex < selectors.length)
              for (fiber = fiber.child;fiber !== null; )
                hostRoot.push(fiber, selectorIndex), fiber = fiber.sibling;
          }
        }
        if (maxSelectorIndex < selectors.length) {
          for (hostRoot = [];maxSelectorIndex < selectors.length; maxSelectorIndex++)
            hostRoot.push(selectorToString(selectors[maxSelectorIndex]));
          return `findAllNodes was able to match part of the selector:
  ` + (matchedNames.join(" > ") + `

No matching component was found for:
  `) + hostRoot.join(" > ");
        }
        return null;
      };
      exports2.getPublicRootInstance = function(container) {
        container = container.current;
        if (!container.child)
          return null;
        switch (container.child.tag) {
          case 27:
          case 5:
            return getPublicInstance(container.child.stateNode);
          default:
            return container.child.stateNode;
        }
      };
      exports2.injectIntoDevTools = function() {
        var internals = {
          bundleType: 0,
          version: rendererVersion,
          rendererPackageName,
          currentDispatcherRef: ReactSharedInternals,
          reconcilerVersion: "19.3.0"
        };
        extraDevToolsConfig !== null && (internals.rendererConfig = extraDevToolsConfig);
        if (typeof __REACT_DEVTOOLS_GLOBAL_HOOK__ === "undefined")
          internals = false;
        else {
          var hook = __REACT_DEVTOOLS_GLOBAL_HOOK__;
          if (hook.isDisabled || !hook.supportsFiber)
            internals = true;
          else {
            try {
              rendererID = hook.inject(internals), injectedHook = hook;
            } catch (err) {}
            internals = hook.checkDCE ? true : false;
          }
        }
        return internals;
      };
      exports2.isAlreadyRendering = function() {
        return (executionContext & 6) !== 0;
      };
      exports2.observeVisibleRects = function(hostRoot, selectors, callback, options) {
        if (!supportsTestSelectors)
          throw Error(formatProdErrorMessage(363));
        hostRoot = findAllNodes(hostRoot, selectors);
        var disconnect = setupIntersectionObserver(hostRoot, callback, options).disconnect;
        return {
          disconnect: function() {
            disconnect();
          }
        };
      };
      exports2.shouldError = function() {
        return null;
      };
      exports2.shouldSuspend = function() {
        return false;
      };
      exports2.startHostTransition = function(formFiber, pendingState, action, formData) {
        if (formFiber.tag !== 5)
          throw Error(formatProdErrorMessage(476));
        var queue = ensureFormComponentIsStateful(formFiber).queue;
        startTransition(formFiber, queue, pendingState, NotPendingTransition, action === null ? noop : function() {
          var stateHook = ensureFormComponentIsStateful(formFiber);
          stateHook.next === null && (stateHook = formFiber.alternate.memoizedState);
          dispatchSetStateInternal(formFiber, stateHook.next.queue, {}, requestUpdateLane());
          return action(formData);
        });
      };
      exports2.updateContainer = function(element, container, parentComponent, callback) {
        var current = container.current, lane = requestUpdateLane();
        updateContainerImpl(current, lane, element, container, parentComponent, callback);
        return lane;
      };
      exports2.updateContainerSync = function(element, container, parentComponent, callback) {
        updateContainerImpl(container.current, 2, element, container, parentComponent, callback);
        return 2;
      };
      return exports2;
    };
    module.exports.default = module.exports;
    Object.defineProperty(module.exports, "__esModule", { value: true });
  });

  // ../../node_modules/.bun/react-reconciler@0.34.0+0441d6f8f57aca1e/node_modules/react-reconciler/index.js
  var require_react_reconciler = __commonJS(function(exports, module) {
    if (true) {
      module.exports = require_react_reconciler_production();
    }
  });

  // ../../node_modules/.bun/react@19.3.1/node_modules/react/cjs/react-jsx-runtime.production.js
  var require_react_jsx_runtime_production = __commonJS(function(exports) {
    var REACT_ELEMENT_TYPE = Symbol.for("react.transitional.element");
    var REACT_FRAGMENT_TYPE = Symbol.for("react.fragment");
    function jsxProd(type, config, maybeKey) {
      var key = null;
      maybeKey !== undefined && (key = "" + maybeKey);
      config.key !== undefined && (key = "" + config.key);
      if ("key" in config) {
        maybeKey = {};
        for (var propName in config)
          propName !== "key" && (maybeKey[propName] = config[propName]);
      } else
        maybeKey = config;
      config = maybeKey.ref;
      return {
        $$typeof: REACT_ELEMENT_TYPE,
        type,
        key,
        ref: config !== undefined ? config : null,
        props: maybeKey
      };
    }
    exports.Fragment = REACT_FRAGMENT_TYPE;
    exports.jsx = jsxProd;
    exports.jsxs = jsxProd;
  });

  // ../../node_modules/.bun/react@19.3.1/node_modules/react/jsx-runtime.js
  var require_jsx_runtime = __commonJS(function(exports, module) {
    if (true) {
      module.exports = require_react_jsx_runtime_production();
    }
  });

  // ../../js/engine/src/layers.ts
  function checkLayer(layer) {
    if (!Number.isInteger(layer) || layer < 0 || layer > 31)
      throw new RangeError(`layer must be 0 to 31, got ${layer}`);
    return layer;
  }

  // ../../js/engine/src/math.ts
  var DEG = Math.PI / 180;
  var Mathf = {
    PI: Math.PI,
    Deg2Rad: DEG,
    Rad2Deg: 1 / DEG,
    Epsilon: 0.000001,
    clamp: (v, min, max) => Math.min(max, Math.max(min, v)),
    clamp01: (v) => Math.min(1, Math.max(0, v)),
    lerp: (a, b, t) => a + (b - a) * Math.min(1, Math.max(0, t)),
    lerpUnclamped: (a, b, t) => a + (b - a) * t,
    inverseLerp: (a, b, v) => a === b ? 0 : Math.min(1, Math.max(0, (v - a) / (b - a))),
    moveTowards: (current, target, maxDelta) => Math.abs(target - current) <= maxDelta ? target : current + Math.sign(target - current) * maxDelta,
    smoothStep: (from, to, t) => {
      const x = Math.min(1, Math.max(0, t));
      return from + (to - from) * x * x * (3 - 2 * x);
    },
    deltaAngle: (a, b) => {
      const d = ((b - a) % 360 + 540) % 360 - 180;
      return d === -180 ? 180 : d;
    },
    repeat: (t, length) => t - Math.floor(t / length) * length,
    pingPong: (t, length) => {
      const r = t - Math.floor(t / (length * 2)) * length * 2;
      return length - Math.abs(r - length);
    },
    approximately: (a, b) => Math.abs(a - b) < Math.max(0.000001 * Math.max(Math.abs(a), Math.abs(b)), 0.000001),
    sign: (v) => v >= 0 ? 1 : -1,
    lerpAngle: (a, b, t) => a + Mathf.deltaAngle(a, b) * Math.min(1, Math.max(0, t)),
    moveTowardsAngle: (current, target, maxDelta) => {
      const d = Mathf.deltaAngle(current, target);
      return Math.abs(d) <= maxDelta ? current + d : current + Math.sign(d) * maxDelta;
    },
    smoothDamp: (current, target, velocity, smoothTime, deltaTime, maxSpeed = Infinity) => {
      const st = Math.max(0.0001, smoothTime);
      const omega = 2 / st;
      const x = omega * deltaTime;
      const exp = 1 / (1 + x + 0.48 * x * x + 0.235 * x * x * x);
      const maxChange = maxSpeed * st;
      const change = Math.min(maxChange, Math.max(-maxChange, current - target));
      const goal = current - change;
      const temp = (velocity + omega * change) * deltaTime;
      let v = (velocity - omega * temp) * exp;
      let out = goal + (change + temp) * exp;
      if (target - current > 0 === out > target) {
        out = target;
        v = (out - target) / deltaTime;
      }
      return [out, v];
    },
    perlinNoise: (x, y) => perlin2(x, y) * 0.5 + 0.5,
    isPowerOfTwo: (v) => v > 0 && (v & v - 1) === 0,
    nextPowerOfTwo: (v) => v <= 1 ? 1 : 2 ** Math.ceil(Math.log2(v)),
    closestPowerOfTwo: (v) => v <= 1 ? 1 : 2 ** Math.round(Math.log2(v)),
    gammaToLinearSpace: (c) => c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4,
    linearToGammaSpace: (c) => c <= 0.0031308 ? c * 12.92 : 1.055 * c ** (1 / 2.4) - 0.055
  };
  var PERM = (() => {
    const p = [
      151,
      160,
      137,
      91,
      90,
      15,
      131,
      13,
      201,
      95,
      96,
      53,
      194,
      233,
      7,
      225,
      140,
      36,
      103,
      30,
      69,
      142,
      8,
      99,
      37,
      240,
      21,
      10,
      23,
      190,
      6,
      148,
      247,
      120,
      234,
      75,
      0,
      26,
      197,
      62,
      94,
      252,
      219,
      203,
      117,
      35,
      11,
      32,
      57,
      177,
      33,
      88,
      237,
      149,
      56,
      87,
      174,
      20,
      125,
      136,
      171,
      168,
      68,
      175,
      74,
      165,
      71,
      134,
      139,
      48,
      27,
      166,
      77,
      146,
      158,
      231,
      83,
      111,
      229,
      122,
      60,
      211,
      133,
      230,
      220,
      105,
      92,
      41,
      55,
      46,
      245,
      40,
      244,
      102,
      143,
      54,
      65,
      25,
      63,
      161,
      1,
      216,
      80,
      73,
      209,
      76,
      132,
      187,
      208,
      89,
      18,
      169,
      200,
      196,
      135,
      130,
      116,
      188,
      159,
      86,
      164,
      100,
      109,
      198,
      173,
      186,
      3,
      64,
      52,
      217,
      226,
      250,
      124,
      123,
      5,
      202,
      38,
      147,
      118,
      126,
      255,
      82,
      85,
      212,
      207,
      206,
      59,
      227,
      47,
      16,
      58,
      17,
      182,
      189,
      28,
      42,
      223,
      183,
      170,
      213,
      119,
      248,
      152,
      2,
      44,
      154,
      163,
      70,
      221,
      153,
      101,
      155,
      167,
      43,
      172,
      9,
      129,
      22,
      39,
      253,
      19,
      98,
      108,
      110,
      79,
      113,
      224,
      232,
      178,
      185,
      112,
      104,
      218,
      246,
      97,
      228,
      251,
      34,
      242,
      193,
      238,
      210,
      144,
      12,
      191,
      179,
      162,
      241,
      81,
      51,
      145,
      235,
      249,
      14,
      239,
      107,
      49,
      192,
      214,
      31,
      181,
      199,
      106,
      157,
      184,
      84,
      204,
      176,
      115,
      121,
      50,
      45,
      127,
      4,
      150,
      254,
      138,
      236,
      205,
      93,
      222,
      114,
      67,
      29,
      24,
      72,
      243,
      141,
      128,
      195,
      78,
      66,
      215,
      61,
      156,
      180
    ];
    return [...p, ...p];
  })();
  function perlin2(x, y) {
    const xi = Math.floor(x) & 255;
    const yi = Math.floor(y) & 255;
    const xf = x - Math.floor(x);
    const yf = y - Math.floor(y);
    const fade = (t) => t * t * t * (t * (t * 6 - 15) + 10);
    const grad = (h, gx, gy) => ((h & 1) === 0 ? gx : -gx) + ((h & 2) === 0 ? gy : -gy);
    const u = fade(xf);
    const v = fade(yf);
    const aa = PERM[PERM[xi] + yi];
    const ab = PERM[PERM[xi] + yi + 1];
    const ba = PERM[PERM[xi + 1] + yi];
    const bb = PERM[PERM[xi + 1] + yi + 1];
    const x1 = grad(aa, xf, yf) + (grad(ba, xf - 1, yf) - grad(aa, xf, yf)) * u;
    const x2 = grad(ab, xf, yf - 1) + (grad(bb, xf - 1, yf - 1) - grad(ab, xf, yf - 1)) * u;
    return x1 + (x2 - x1) * v;
  }

  class Vector2 {
    x;
    y;
    constructor(x = 0, y = 0) {
      this.x = x;
      this.y = y;
    }
    static zero = new Vector2(0, 0);
    static one = new Vector2(1, 1);
    static up = new Vector2(0, 1);
    static down = new Vector2(0, -1);
    static left = new Vector2(-1, 0);
    static right = new Vector2(1, 0);
    static from(v) {
      return v instanceof Vector2 ? v : new Vector2(v.x, v.y);
    }
    add(v) {
      return new Vector2(this.x + v.x, this.y + v.y);
    }
    sub(v) {
      return new Vector2(this.x - v.x, this.y - v.y);
    }
    scale(s) {
      return new Vector2(this.x * s, this.y * s);
    }
    mul(v) {
      return new Vector2(this.x * v.x, this.y * v.y);
    }
    dot(v) {
      return this.x * v.x + this.y * v.y;
    }
    get magnitude() {
      return Math.hypot(this.x, this.y);
    }
    get sqrMagnitude() {
      return this.x * this.x + this.y * this.y;
    }
    get normalized() {
      const m = this.magnitude;
      return m > 0 ? this.scale(1 / m) : Vector2.zero;
    }
    distance(v) {
      return Math.hypot(this.x - v.x, this.y - v.y);
    }
    lerp(v, t) {
      const k = Mathf.clamp01(t);
      return new Vector2(this.x + (v.x - this.x) * k, this.y + (v.y - this.y) * k);
    }
    moveTowards(target, maxDistance) {
      const d = Vector2.from(target).sub(this);
      const m = d.magnitude;
      return m <= maxDistance || m === 0 ? Vector2.from(target) : this.add(d.scale(maxDistance / m));
    }
    rotate(degrees) {
      const c = Math.cos(degrees * DEG);
      const s = Math.sin(degrees * DEG);
      return new Vector2(this.x * c - this.y * s, this.x * s + this.y * c);
    }
    static angle(a, b) {
      const d = Vector2.from(a).magnitude * Vector2.from(b).magnitude;
      return d === 0 ? 0 : Math.acos(Mathf.clamp(Vector2.from(a).dot(b) / d, -1, 1)) / DEG;
    }
    static signedAngle(a, b) {
      return Math.atan2(a.x * b.y - a.y * b.x, a.x * b.x + a.y * b.y) / DEG;
    }
    equals(v, epsilon = 0.000001) {
      return Math.abs(this.x - v.x) <= epsilon && Math.abs(this.y - v.y) <= epsilon;
    }
    toVector3(z = 0) {
      return new Vector3(this.x, this.y, z);
    }
    toArray() {
      return [this.x, this.y];
    }
    toString() {
      return `(${this.x}, ${this.y})`;
    }
  }

  class Vector3 {
    x;
    y;
    z;
    constructor(x = 0, y = 0, z = 0) {
      this.x = x;
      this.y = y;
      this.z = z;
    }
    static zero = new Vector3(0, 0, 0);
    static one = new Vector3(1, 1, 1);
    static up = new Vector3(0, 1, 0);
    static down = new Vector3(0, -1, 0);
    static left = new Vector3(-1, 0, 0);
    static right = new Vector3(1, 0, 0);
    static forward = new Vector3(0, 0, -1);
    static back = new Vector3(0, 0, 1);
    static from(v) {
      return v instanceof Vector3 ? v : new Vector3(v.x, v.y, v.z);
    }
    add(v) {
      return new Vector3(this.x + v.x, this.y + v.y, this.z + v.z);
    }
    sub(v) {
      return new Vector3(this.x - v.x, this.y - v.y, this.z - v.z);
    }
    scale(s) {
      return new Vector3(this.x * s, this.y * s, this.z * s);
    }
    mul(v) {
      return new Vector3(this.x * v.x, this.y * v.y, this.z * v.z);
    }
    div(v) {
      return new Vector3(this.x / v.x, this.y / v.y, this.z / v.z);
    }
    negate() {
      return new Vector3(-this.x, -this.y, -this.z);
    }
    dot(v) {
      return this.x * v.x + this.y * v.y + this.z * v.z;
    }
    cross(v) {
      return new Vector3(this.y * v.z - this.z * v.y, this.z * v.x - this.x * v.z, this.x * v.y - this.y * v.x);
    }
    get magnitude() {
      return Math.hypot(this.x, this.y, this.z);
    }
    get sqrMagnitude() {
      return this.x * this.x + this.y * this.y + this.z * this.z;
    }
    get normalized() {
      const m = this.magnitude;
      return m > 0 ? this.scale(1 / m) : Vector3.zero;
    }
    distance(v) {
      return Math.hypot(this.x - v.x, this.y - v.y, this.z - v.z);
    }
    lerp(v, t) {
      const k = Mathf.clamp01(t);
      return new Vector3(this.x + (v.x - this.x) * k, this.y + (v.y - this.y) * k, this.z + (v.z - this.z) * k);
    }
    moveTowards(target, maxDistance) {
      const d = Vector3.from(target).sub(this);
      const m = d.magnitude;
      return m <= maxDistance || m === 0 ? Vector3.from(target) : this.add(d.scale(maxDistance / m));
    }
    project(onto) {
      const n = Vector3.from(onto);
      const sq = n.sqrMagnitude;
      return sq === 0 ? Vector3.zero : n.scale(this.dot(n) / sq);
    }
    projectOnPlane(normal) {
      return this.sub(this.project(normal));
    }
    reflect(normal) {
      return this.sub(Vector3.from(normal).scale(2 * this.dot(normal)));
    }
    clampMagnitude(max) {
      const m = this.magnitude;
      return m > max ? this.scale(max / m) : this;
    }
    static angle(a, b) {
      const va = Vector3.from(a);
      const d = va.magnitude * Vector3.from(b).magnitude;
      return d === 0 ? 0 : Math.acos(Mathf.clamp(va.dot(b) / d, -1, 1)) / DEG;
    }
    equals(v, epsilon = 0.000001) {
      return Math.abs(this.x - v.x) <= epsilon && Math.abs(this.y - v.y) <= epsilon && Math.abs(this.z - v.z) <= epsilon;
    }
    toVector2() {
      return new Vector2(this.x, this.y);
    }
    toArray() {
      return [this.x, this.y, this.z];
    }
    toString() {
      return `(${this.x}, ${this.y}, ${this.z})`;
    }
  }

  class Vector4 {
    x;
    y;
    z;
    w;
    constructor(x = 0, y = 0, z = 0, w = 0) {
      this.x = x;
      this.y = y;
      this.z = z;
      this.w = w;
    }
    static zero = new Vector4;
    static one = new Vector4(1, 1, 1, 1);
    static fromVector3(v, w = 1) {
      return new Vector4(v.x, v.y, v.z, w);
    }
    add(v) {
      return new Vector4(this.x + v.x, this.y + v.y, this.z + v.z, this.w + v.w);
    }
    sub(v) {
      return new Vector4(this.x - v.x, this.y - v.y, this.z - v.z, this.w - v.w);
    }
    scale(s) {
      return new Vector4(this.x * s, this.y * s, this.z * s, this.w * s);
    }
    mul(v) {
      return new Vector4(this.x * v.x, this.y * v.y, this.z * v.z, this.w * v.w);
    }
    dot(v) {
      return this.x * v.x + this.y * v.y + this.z * v.z + this.w * v.w;
    }
    get magnitude() {
      return Math.hypot(this.x, this.y, this.z, this.w);
    }
    get normalized() {
      const m = this.magnitude;
      return m > 0 ? this.scale(1 / m) : Vector4.zero;
    }
    lerp(v, t) {
      return this.add(v.sub(this).scale(Mathf.clamp01(t)));
    }
    equals(v, epsilon = 0.000001) {
      return this.sub(v).magnitude <= epsilon;
    }
    toVector3() {
      return new Vector3(this.x, this.y, this.z);
    }
    toArray() {
      return [this.x, this.y, this.z, this.w];
    }
    toString() {
      return `(${this.x}, ${this.y}, ${this.z}, ${this.w})`;
    }
  }
  class Vector2Int {
    x;
    y;
    constructor(x = 0, y = 0) {
      this.x = Math.trunc(x);
      this.y = Math.trunc(y);
    }
    static zero = new Vector2Int(0, 0);
    static one = new Vector2Int(1, 1);
    static up = new Vector2Int(0, 1);
    static down = new Vector2Int(0, -1);
    static left = new Vector2Int(-1, 0);
    static right = new Vector2Int(1, 0);
    static floor(v) {
      return new Vector2Int(Math.floor(v.x), Math.floor(v.y));
    }
    static round(v) {
      return new Vector2Int(Math.round(v.x), Math.round(v.y));
    }
    add(v) {
      return new Vector2Int(this.x + v.x, this.y + v.y);
    }
    sub(v) {
      return new Vector2Int(this.x - v.x, this.y - v.y);
    }
    scale(s) {
      return new Vector2Int(this.x * s, this.y * s);
    }
    manhattan(v) {
      return Math.abs(this.x - v.x) + Math.abs(this.y - v.y);
    }
    equals(v) {
      return this.x === v.x && this.y === v.y;
    }
    toVector2() {
      return new Vector2(this.x, this.y);
    }
    toString() {
      return `(${this.x}, ${this.y})`;
    }
  }

  class Vector3Int {
    x;
    y;
    z;
    constructor(x = 0, y = 0, z = 0) {
      this.x = Math.trunc(x);
      this.y = Math.trunc(y);
      this.z = Math.trunc(z);
    }
    static zero = new Vector3Int(0, 0, 0);
    static one = new Vector3Int(1, 1, 1);
    static floor(v) {
      return new Vector3Int(Math.floor(v.x), Math.floor(v.y), Math.floor(v.z));
    }
    static round(v) {
      return new Vector3Int(Math.round(v.x), Math.round(v.y), Math.round(v.z));
    }
    add(v) {
      return new Vector3Int(this.x + v.x, this.y + v.y, this.z + v.z);
    }
    sub(v) {
      return new Vector3Int(this.x - v.x, this.y - v.y, this.z - v.z);
    }
    equals(v) {
      return this.x === v.x && this.y === v.y && this.z === v.z;
    }
    toVector3() {
      return new Vector3(this.x, this.y, this.z);
    }
    toString() {
      return `(${this.x}, ${this.y}, ${this.z})`;
    }
  }

  class Quaternion {
    x;
    y;
    z;
    w;
    constructor(x = 0, y = 0, z = 0, w = 1) {
      this.x = x;
      this.y = y;
      this.z = z;
      this.w = w;
    }
    static identity = new Quaternion(0, 0, 0, 1);
    static from(q) {
      return q instanceof Quaternion ? q : new Quaternion(q.x, q.y, q.z, q.w);
    }
    static angleAxis(degrees, axis) {
      const a = Vector3.from(axis).normalized;
      const half = degrees * DEG / 2;
      const s = Math.sin(half);
      return new Quaternion(a.x * s, a.y * s, a.z * s, Math.cos(half));
    }
    static euler(x, y = 0, z = 0) {
      const a = typeof x === "number" ? new Vector3(x, y, z) : x;
      return Quaternion.angleAxis(a.y, Vector3.up).mul(Quaternion.angleAxis(a.x, Vector3.right)).mul(Quaternion.angleAxis(a.z, new Vector3(0, 0, 1)));
    }
    get eulerAngles() {
      const { x, y, z, w } = this;
      const m12 = 2 * (y * z - w * x);
      const sx = Mathf.clamp(-m12, -1, 1);
      const ax = Math.asin(sx);
      let ay;
      let az;
      if (Math.abs(sx) < 0.9999) {
        ay = Math.atan2(2 * (x * z + w * y), 1 - 2 * (x * x + y * y));
        az = Math.atan2(2 * (x * y + w * z), 1 - 2 * (x * x + z * z));
      } else {
        ay = Math.atan2(-2 * (x * z - w * y), 1 - 2 * (y * y + z * z));
        az = 0;
      }
      const wrap = (r) => Mathf.repeat(r / DEG, 360);
      return new Vector3(wrap(ax), wrap(ay), wrap(az));
    }
    static lookRotation(direction, up = Vector3.up) {
      const f = Vector3.from(direction).normalized;
      if (f.sqrMagnitude === 0)
        return Quaternion.identity;
      let r = f.cross(up).normalized;
      if (r.sqrMagnitude === 0)
        r = f.cross(Math.abs(f.x) < 0.9 ? Vector3.right : Vector3.up).normalized;
      const u = r.cross(f);
      const [m00, m01, m02] = [r.x, u.x, -f.x];
      const [m10, m11, m12] = [r.y, u.y, -f.y];
      const [m20, m21, m22] = [r.z, u.z, -f.z];
      const trace = m00 + m11 + m22;
      if (trace > 0) {
        const s = Math.sqrt(trace + 1) * 2;
        return new Quaternion((m21 - m12) / s, (m02 - m20) / s, (m10 - m01) / s, s / 4);
      }
      if (m00 > m11 && m00 > m22) {
        const s = Math.sqrt(1 + m00 - m11 - m22) * 2;
        return new Quaternion(s / 4, (m01 + m10) / s, (m02 + m20) / s, (m21 - m12) / s);
      }
      if (m11 > m22) {
        const s = Math.sqrt(1 + m11 - m00 - m22) * 2;
        return new Quaternion((m01 + m10) / s, s / 4, (m12 + m21) / s, (m02 - m20) / s);
      }
      const s = Math.sqrt(1 + m22 - m00 - m11) * 2;
      return new Quaternion((m02 + m20) / s, (m12 + m21) / s, s / 4, (m10 - m01) / s);
    }
    static fromToRotation(from, to) {
      const a = Vector3.from(from).normalized;
      const b = Vector3.from(to).normalized;
      const d = a.dot(b);
      if (d < -0.999999) {
        let axis = Vector3.right.cross(a);
        if (axis.sqrMagnitude < 0.000001)
          axis = Vector3.up.cross(a);
        return Quaternion.angleAxis(180, axis);
      }
      const c = a.cross(b);
      return new Quaternion(c.x, c.y, c.z, 1 + d).normalized;
    }
    mul(q) {
      return new Quaternion(this.w * q.x + this.x * q.w + this.y * q.z - this.z * q.y, this.w * q.y - this.x * q.z + this.y * q.w + this.z * q.x, this.w * q.z + this.x * q.y - this.y * q.x + this.z * q.w, this.w * q.w - this.x * q.x - this.y * q.y - this.z * q.z);
    }
    inverse() {
      return new Quaternion(-this.x, -this.y, -this.z, this.w);
    }
    rotate(v) {
      const q = new Vector3(this.x, this.y, this.z);
      const t = q.cross(v).scale(2);
      return Vector3.from(v).add(t.scale(this.w)).add(q.cross(t));
    }
    get normalized() {
      const len = Math.hypot(this.x, this.y, this.z, this.w);
      return len > 0 ? new Quaternion(this.x / len, this.y / len, this.z / len, this.w / len) : Quaternion.identity;
    }
    slerp(q, t) {
      const k = Mathf.clamp01(t);
      let { x, y, z, w } = q;
      let cos = this.x * x + this.y * y + this.z * z + this.w * w;
      if (cos < 0) {
        [x, y, z, w, cos] = [-x, -y, -z, -w, -cos];
      }
      if (cos > 0.9995) {
        return new Quaternion(this.x + (x - this.x) * k, this.y + (y - this.y) * k, this.z + (z - this.z) * k, this.w + (w - this.w) * k).normalized;
      }
      const angle = Math.acos(cos);
      const a = Math.sin((1 - k) * angle) / Math.sin(angle);
      const b = Math.sin(k * angle) / Math.sin(angle);
      return new Quaternion(this.x * a + x * b, this.y * a + y * b, this.z * a + z * b, this.w * a + w * b);
    }
    rotateTowards(target, maxDegrees) {
      const angle = Quaternion.angle(this, target);
      return angle === 0 ? Quaternion.from(target) : this.slerp(target, Math.min(1, maxDegrees / angle));
    }
    static angle(a, b) {
      const d = Math.abs(a.x * b.x + a.y * b.y + a.z * b.z + a.w * b.w);
      return 2 * Math.acos(Math.min(1, d)) / DEG;
    }
    toArray() {
      return [this.x, this.y, this.z, this.w];
    }
  }

  class Matrix4x4 {
    m;
    constructor(m) {
      if (m.length !== 16)
        throw new Error("Matrix4x4 needs 16 numbers");
      this.m = m;
    }
    static identity = new Matrix4x4([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]);
    get(row, col) {
      return this.m[col * 4 + row];
    }
    static trs(position, rotation, scale = Vector3.one) {
      const { x, y, z, w } = rotation;
      const [xx, yy, zz] = [x * x, y * y, z * z];
      const [xy, xz, yz, wx, wy, wz] = [x * y, x * z, y * z, w * x, w * y, w * z];
      return new Matrix4x4([
        (1 - 2 * (yy + zz)) * scale.x,
        2 * (xy + wz) * scale.x,
        2 * (xz - wy) * scale.x,
        0,
        2 * (xy - wz) * scale.y,
        (1 - 2 * (xx + zz)) * scale.y,
        2 * (yz + wx) * scale.y,
        0,
        2 * (xz + wy) * scale.z,
        2 * (yz - wx) * scale.z,
        (1 - 2 * (xx + yy)) * scale.z,
        0,
        position.x,
        position.y,
        position.z,
        1
      ]);
    }
    static perspective(fovDegrees, aspect, near, far) {
      const f = 1 / Math.tan(fovDegrees * DEG / 2);
      const r = far / (near - far);
      return new Matrix4x4([f / aspect, 0, 0, 0, 0, f, 0, 0, 0, 0, r, -1, 0, 0, r * near, 0]);
    }
    static orthographic(left, right, bottom, top, near, far) {
      const rw = 1 / (right - left);
      const rh = 1 / (top - bottom);
      const r = 1 / (near - far);
      return new Matrix4x4([2 * rw, 0, 0, 0, 0, 2 * rh, 0, 0, 0, 0, r, 0, -(left + right) * rw, -(top + bottom) * rh, r * near, 1]);
    }
    mul(other) {
      const a = this.m;
      const b = other.m;
      const out = new Array(16);
      for (let c = 0;c < 4; c++) {
        for (let r = 0;r < 4; r++) {
          out[c * 4 + r] = a[r] * b[c * 4] + a[4 + r] * b[c * 4 + 1] + a[8 + r] * b[c * 4 + 2] + a[12 + r] * b[c * 4 + 3];
        }
      }
      return new Matrix4x4(out);
    }
    get inverse() {
      const m = this.m;
      const inv = new Array(16);
      inv[0] = m[5] * m[10] * m[15] - m[5] * m[11] * m[14] - m[9] * m[6] * m[15] + m[9] * m[7] * m[14] + m[13] * m[6] * m[11] - m[13] * m[7] * m[10];
      inv[4] = -m[4] * m[10] * m[15] + m[4] * m[11] * m[14] + m[8] * m[6] * m[15] - m[8] * m[7] * m[14] - m[12] * m[6] * m[11] + m[12] * m[7] * m[10];
      inv[8] = m[4] * m[9] * m[15] - m[4] * m[11] * m[13] - m[8] * m[5] * m[15] + m[8] * m[7] * m[13] + m[12] * m[5] * m[11] - m[12] * m[7] * m[9];
      inv[12] = -m[4] * m[9] * m[14] + m[4] * m[10] * m[13] + m[8] * m[5] * m[14] - m[8] * m[6] * m[13] - m[12] * m[5] * m[10] + m[12] * m[6] * m[9];
      inv[1] = -m[1] * m[10] * m[15] + m[1] * m[11] * m[14] + m[9] * m[2] * m[15] - m[9] * m[3] * m[14] - m[13] * m[2] * m[11] + m[13] * m[3] * m[10];
      inv[5] = m[0] * m[10] * m[15] - m[0] * m[11] * m[14] - m[8] * m[2] * m[15] + m[8] * m[3] * m[14] + m[12] * m[2] * m[11] - m[12] * m[3] * m[10];
      inv[9] = -m[0] * m[9] * m[15] + m[0] * m[11] * m[13] + m[8] * m[1] * m[15] - m[8] * m[3] * m[13] - m[12] * m[1] * m[11] + m[12] * m[3] * m[9];
      inv[13] = m[0] * m[9] * m[14] - m[0] * m[10] * m[13] - m[8] * m[1] * m[14] + m[8] * m[2] * m[13] + m[12] * m[1] * m[10] - m[12] * m[2] * m[9];
      inv[2] = m[1] * m[6] * m[15] - m[1] * m[7] * m[14] - m[5] * m[2] * m[15] + m[5] * m[3] * m[14] + m[13] * m[2] * m[7] - m[13] * m[3] * m[6];
      inv[6] = -m[0] * m[6] * m[15] + m[0] * m[7] * m[14] + m[4] * m[2] * m[15] - m[4] * m[3] * m[14] - m[12] * m[2] * m[7] + m[12] * m[3] * m[6];
      inv[10] = m[0] * m[5] * m[15] - m[0] * m[7] * m[13] - m[4] * m[1] * m[15] + m[4] * m[3] * m[13] + m[12] * m[1] * m[7] - m[12] * m[3] * m[5];
      inv[14] = -m[0] * m[5] * m[14] + m[0] * m[6] * m[13] + m[4] * m[1] * m[14] - m[4] * m[2] * m[13] - m[12] * m[1] * m[6] + m[12] * m[2] * m[5];
      inv[3] = -m[1] * m[6] * m[11] + m[1] * m[7] * m[10] + m[5] * m[2] * m[11] - m[5] * m[3] * m[10] - m[9] * m[2] * m[7] + m[9] * m[3] * m[6];
      inv[7] = m[0] * m[6] * m[11] - m[0] * m[7] * m[10] - m[4] * m[2] * m[11] + m[4] * m[3] * m[10] + m[8] * m[2] * m[7] - m[8] * m[3] * m[6];
      inv[11] = -m[0] * m[5] * m[11] + m[0] * m[7] * m[9] + m[4] * m[1] * m[11] - m[4] * m[3] * m[9] - m[8] * m[1] * m[7] + m[8] * m[3] * m[5];
      inv[15] = m[0] * m[5] * m[10] - m[0] * m[6] * m[9] - m[4] * m[1] * m[10] + m[4] * m[2] * m[9] + m[8] * m[1] * m[6] - m[8] * m[2] * m[5];
      const det = m[0] * inv[0] + m[1] * inv[4] + m[2] * inv[8] + m[3] * inv[12];
      if (Math.abs(det) < 0.000000000001)
        return Matrix4x4.identity;
      return new Matrix4x4(inv.map((v) => v / det));
    }
    get determinant() {
      const m = this.m;
      const sub = (a, b, c, d, e, f, g, h, i) => a * (e * i - f * h) - b * (d * i - f * g) + c * (d * h - e * g);
      return m[0] * sub(m[5], m[9], m[13], m[6], m[10], m[14], m[7], m[11], m[15]) - m[4] * sub(m[1], m[9], m[13], m[2], m[10], m[14], m[3], m[11], m[15]) + m[8] * sub(m[1], m[5], m[13], m[2], m[6], m[14], m[3], m[7], m[15]) - m[12] * sub(m[1], m[5], m[9], m[2], m[6], m[10], m[3], m[7], m[11]);
    }
    multiplyPoint(p) {
      const m = this.m;
      const x = m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12];
      const y = m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13];
      const z = m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14];
      const w = m[3] * p.x + m[7] * p.y + m[11] * p.z + m[15];
      return w !== 0 && w !== 1 ? new Vector3(x / w, y / w, z / w) : new Vector3(x, y, z);
    }
    multiplyPoint3x4(p) {
      const m = this.m;
      return new Vector3(m[0] * p.x + m[4] * p.y + m[8] * p.z + m[12], m[1] * p.x + m[5] * p.y + m[9] * p.z + m[13], m[2] * p.x + m[6] * p.y + m[10] * p.z + m[14]);
    }
    multiplyVector(v) {
      const m = this.m;
      return new Vector3(m[0] * v.x + m[4] * v.y + m[8] * v.z, m[1] * v.x + m[5] * v.y + m[9] * v.z, m[2] * v.x + m[6] * v.y + m[10] * v.z);
    }
    multiplyVector4(v) {
      const m = this.m;
      return new Vector4(m[0] * v.x + m[4] * v.y + m[8] * v.z + m[12] * v.w, m[1] * v.x + m[5] * v.y + m[9] * v.z + m[13] * v.w, m[2] * v.x + m[6] * v.y + m[10] * v.z + m[14] * v.w, m[3] * v.x + m[7] * v.y + m[11] * v.z + m[15] * v.w);
    }
    get transpose() {
      const out = new Array(16);
      for (let r = 0;r < 4; r++)
        for (let c = 0;c < 4; c++)
          out[r * 4 + c] = this.m[c * 4 + r];
      return new Matrix4x4(out);
    }
  }

  class Ray {
    origin;
    direction;
    constructor(origin, direction) {
      this.origin = Vector3.from(origin);
      this.direction = Vector3.from(direction).normalized;
    }
    getPoint(distance) {
      return this.origin.add(this.direction.scale(distance));
    }
  }
  class Rect {
    x;
    y;
    width;
    height;
    constructor(x = 0, y = 0, width = 0, height = 0) {
      this.x = x;
      this.y = y;
      this.width = width;
      this.height = height;
    }
    static minMax(min, max) {
      return new Rect(min.x, min.y, max.x - min.x, max.y - min.y);
    }
    get xMin() {
      return this.x;
    }
    get yMin() {
      return this.y;
    }
    get xMax() {
      return this.x + this.width;
    }
    get yMax() {
      return this.y + this.height;
    }
    get center() {
      return new Vector2(this.x + this.width / 2, this.y + this.height / 2);
    }
    get size() {
      return new Vector2(this.width, this.height);
    }
    contains(p) {
      return p.x >= this.x && p.x < this.xMax && p.y >= this.y && p.y < this.yMax;
    }
    overlaps(r) {
      return r.xMax > this.x && r.x < this.xMax && r.yMax > this.y && r.y < this.yMax;
    }
  }

  class RectInt extends Rect {
    constructor(x = 0, y = 0, width = 0, height = 0) {
      super(Math.trunc(x), Math.trunc(y), Math.trunc(width), Math.trunc(height));
    }
    *positions() {
      for (let y = this.y;y < this.yMax; y++)
        for (let x = this.x;x < this.xMax; x++)
          yield new Vector2Int(x, y);
    }
  }
  class Color {
    r;
    g;
    b;
    a;
    constructor(r = 0, g = 0, b = 0, a = 1) {
      this.r = r;
      this.g = g;
      this.b = b;
      this.a = a;
    }
    static white = new Color(1, 1, 1);
    static black = new Color(0, 0, 0);
    static red = new Color(1, 0, 0);
    static green = new Color(0, 1, 0);
    static blue = new Color(0, 0, 1);
    static yellow = new Color(1, 0.92, 0.016);
    static clear = new Color(0, 0, 0, 0);
    static hex(hex) {
      let h = hex.replace(/^#/, "");
      if (h.length === 3 || h.length === 4)
        h = [...h].map((c) => c + c).join("");
      const n = (i) => Number.parseInt(h.slice(i, i + 2), 16) / 255;
      const lin = (i) => Mathf.gammaToLinearSpace(n(i));
      return new Color(lin(0), lin(2), lin(4), h.length >= 8 ? n(6) : 1);
    }
    static fromSrgb(r, g, b, a = 1) {
      return new Color(Mathf.gammaToLinearSpace(r), Mathf.gammaToLinearSpace(g), Mathf.gammaToLinearSpace(b), a);
    }
    toHex() {
      const h = (v) => Math.round(Mathf.clamp01(v) * 255).toString(16).padStart(2, "0");
      const g = (v) => h(Mathf.linearToGammaSpace(Mathf.clamp01(v)));
      return `#${g(this.r)}${g(this.g)}${g(this.b)}${h(this.a)}`;
    }
    lerp(c, t) {
      const k = Mathf.clamp01(t);
      return new Color(this.r + (c.r - this.r) * k, this.g + (c.g - this.g) * k, this.b + (c.b - this.b) * k, this.a + (c.a - this.a) * k);
    }
    withAlpha(a) {
      return new Color(this.r, this.g, this.b, a);
    }
    toArray() {
      return [this.r, this.g, this.b, this.a];
    }
  }

  // ../../js/engine/src/scene.ts
  class Component {
    id;
    gameObject;
    get transform() {
      return this.gameObject.transform;
    }
    getComponent(cls) {
      return this.gameObject.getComponent(cls);
    }
    destroy() {
      this.gameObject.removeComponent(this);
    }
  }
  var classes = new Map;
  var names = new Map;
  var starting = new Set;
  var builtIns = new Map;
  var instances = new Map;
  var objects = new Map;
  var roots = [];
  var nextObject = 1;
  var nextComponent = 1 << 24;
  function _registerBuiltIns(components) {
    for (const [name, cls] of Object.entries(components)) {
      builtIns.set(name, cls);
      if (!names.has(cls))
        names.set(cls, name);
    }
  }
  function _classByName(name) {
    return classes.get(name) ?? builtIns.get(name);
  }
  function _patchClasses() {
    const stale = new Set;
    const fresh = new Map;
    for (const instance of instances.values()) {
      const old = instance.constructor;
      const name = names.get(old);
      if (!name || builtIns.get(name) === old)
        continue;
      const cls = classes.get(name);
      if (!cls) {
        stale.add(name);
        continue;
      }
      if (cls === old)
        continue;
      Object.setPrototypeOf(instance, cls.prototype);
      let defaults = fresh.get(cls);
      if (!defaults) {
        defaults = new cls;
        fresh.set(cls, defaults);
      }
      const own = instance;
      for (const key of Object.keys(defaults))
        if (!Object.hasOwn(own, key))
          own[key] = defaults[key];
    }
    return [...stale];
  }
  function _isBuiltIn(cls) {
    const name = names.get(cls);
    return name !== undefined && builtIns.get(name) === cls;
  }
  var restored = new Set;
  function _markRestored(component) {
    restored.add(component.id);
  }
  function _nameOf(cls) {
    return nameOf(cls);
  }
  var sceneHooks = null;
  function _setSceneHooks(hooks) {
    sceneHooks = hooks;
  }
  function nameOf(cls) {
    let name = names.get(cls);
    if (!name) {
      name = cls.name;
      if (!name)
        throw new Error("component classes need a name; register them with register({ ... })");
      names.set(cls, name);
    }
    return name;
  }
  function instantiate(cls, id, object) {
    const instance = new cls;
    Object.assign(instance, { id, gameObject: object });
    instances.set(id, instance);
    object.components.push(instance);
    return instance;
  }
  var vec = (v) => [v.x, v.y, v.z];
  var quat = (q) => [q.x, q.y, q.z, q.w];
  var scaleListeners = [];
  var WORLD = { position: Vector3.zero, rotation: Quaternion.identity, scale: Vector3.one, identity: true };
  function isIdentity(w) {
    const { position: p, rotation: q, scale: s } = w;
    return p.x === 0 && p.y === 0 && p.z === 0 && q.x === 0 && q.y === 0 && q.z === 0 && q.w === 1 && s.x === 1 && s.y === 1 && s.z === 1;
  }

  class Transform {
    gameObject;
    _position = Vector3.zero;
    _rotation = Quaternion.identity;
    _scale = Vector3.one;
    constructor(gameObject) {
      this.gameObject = gameObject;
    }
    send() {
      pendingTransforms.set(this.gameObject.id, this);
    }
    _setLocal(position, rotation, scale) {
      const old = this._scale;
      this._position = position;
      this._rotation = rotation;
      this._scale = scale;
      this.send();
      if (!old.equals(scale))
        for (const f of scaleListeners)
          f(this.gameObject);
    }
    _set(position, rotation, scale) {
      this._position = position;
      this._rotation = rotation;
      this._scale = scale;
    }
    get localPosition() {
      return this._position;
    }
    set localPosition(v) {
      this._position = Vector3.from(v);
      this.send();
    }
    get localRotation() {
      return this._rotation;
    }
    set localRotation(q) {
      this._rotation = Quaternion.from(q).normalized;
      this.send();
    }
    get localScale() {
      return this._scale;
    }
    set localScale(v) {
      const old = this._scale;
      this._scale = Vector3.from(v);
      this.send();
      if (!old.equals(this._scale))
        for (const f of scaleListeners)
          f(this.gameObject);
    }
    get localEulerAngles() {
      return this._rotation.eulerAngles;
    }
    set localEulerAngles(v) {
      this.localRotation = Quaternion.euler(v);
    }
    world() {
      const parent = this.gameObject.parent?.transform;
      if (!parent)
        return { position: this._position, rotation: this._rotation, scale: this._scale, identity: false };
      const p = parent.world();
      if (p.identity || isIdentity(p))
        return { position: this._position, rotation: this._rotation, scale: this._scale, identity: false };
      return {
        position: p.position.add(p.rotation.rotate(p.scale.mul(this._position))),
        rotation: p.rotation.mul(this._rotation),
        scale: p.scale.mul(this._scale),
        identity: false
      };
    }
    parentWorld() {
      return this.gameObject.parent?.transform.world() ?? WORLD;
    }
    get position() {
      const p = this.parentWorld();
      if (p.identity)
        return this._position;
      return p.position.add(p.rotation.rotate(p.scale.mul(this._position)));
    }
    set position(v) {
      const p = this.parentWorld();
      if (p.identity || isIdentity(p)) {
        this._position = Vector3.from(v);
        this.send();
        return;
      }
      this._position = p.rotation.inverse().rotate(Vector3.from(v).sub(p.position)).div(p.scale);
      this.send();
    }
    get rotation() {
      const p = this.parentWorld();
      return p.identity ? this._rotation : p.rotation.mul(this._rotation);
    }
    set rotation(q) {
      const p = this.parentWorld();
      if (p.identity || isIdentity(p)) {
        this._rotation = Quaternion.from(q).normalized;
        this.send();
        return;
      }
      this._rotation = p.rotation.inverse().mul(q).normalized;
      this.send();
    }
    get eulerAngles() {
      return this.rotation.eulerAngles;
    }
    set eulerAngles(v) {
      this.rotation = Quaternion.euler(v);
    }
    get lossyScale() {
      return this.parentWorld().scale.mul(this._scale);
    }
    get forward() {
      return this.rotation.rotate(Vector3.forward);
    }
    get right() {
      return this.rotation.rotate(Vector3.right);
    }
    get up() {
      return this.rotation.rotate(Vector3.up);
    }
    translate(delta, space = "world") {
      const d = space === "self" ? this.rotation.rotate(delta) : Vector3.from(delta);
      this.position = this.position.add(d);
    }
    rotate(euler, space = "self") {
      const q = Quaternion.euler(euler);
      this.rotation = space === "self" ? this.rotation.mul(q) : q.mul(this.rotation);
    }
    lookAt(target, up = Vector3.up) {
      this.rotation = Quaternion.lookRotation(Vector3.from(target).sub(this.position), up);
    }
    transformPoint(p) {
      return this.position.add(this.rotation.rotate(this.lossyScale.mul(p)));
    }
    inverseTransformPoint(p) {
      return this.rotation.inverse().rotate(Vector3.from(p).sub(this.position)).div(this.lossyScale);
    }
    transformDirection(d) {
      return this.rotation.rotate(d);
    }
    get parent() {
      return this.gameObject.parent?.transform ?? null;
    }
  }
  var HideFlags = {
    None: 0,
    HideInHierarchy: 1,
    HideInInspector: 2,
    DontSaveInEditor: 4,
    NotEditable: 8,
    DontSaveInBuild: 16,
    DontUnloadUnusedAsset: 32,
    DontSave: 52,
    HideAndDontSave: 61
  };

  class GameObject {
    id;
    transform;
    components = [];
    children = [];
    _name;
    _parent = null;
    _enabled = true;
    _destroyed = false;
    _layer = 0;
    _scene = null;
    _model = null;
    _modelPart = false;
    _prefab = null;
    hideFlags = 0;
    constructor(name = "GameObject", options = {}) {
      this.id = nextObject++;
      this._name = name;
      this.transform = new Transform(this);
      this.transform._set(Vector3.from(options.position ?? Vector3.zero), options.rotation ? Quaternion.from(options.rotation).normalized : Quaternion.identity, Vector3.from(options.scale ?? Vector3.one));
      objects.set(this.id, this);
      const parent = options.parent ?? null;
      this._parent = parent;
      if (!parent)
        this._scene = sceneHooks?.active() ?? null;
      (parent ? parent.children : roots).push(this);
      const t = this.transform;
      flushTransforms();
      __iron_op("scene.create", this.id, name, parent?.id ?? null, vec(t._position), quat(t._rotation), vec(t._scale));
    }
    static _adopt(id, name) {
      const go = Object.create(GameObject.prototype);
      Object.assign(go, {
        id,
        components: [],
        children: [],
        _name: name,
        _parent: null,
        _enabled: true,
        _destroyed: false,
        _layer: 0,
        _scene: sceneHooks?.active() ?? null,
        _model: null,
        _modelPart: false,
        _prefab: null,
        hideFlags: 0
      });
      Object.assign(go, { transform: new Transform(go) });
      objects.set(id, go);
      roots.push(go);
      return go;
    }
    get name() {
      return this._name;
    }
    set name(name) {
      this._name = name;
      flushTransforms();
      __iron_op("scene.name", this.id, name);
    }
    get layer() {
      return this._layer;
    }
    set layer(layer) {
      this._layer = checkLayer(layer);
      flushTransforms();
      __iron_op("scene.layer", this.id, layer);
    }
    get parent() {
      return this._parent;
    }
    get scene() {
      let root = this;
      while (root._parent)
        root = root._parent;
      return root._scene;
    }
    get root() {
      let root = this;
      while (root._parent)
        root = root._parent;
      return root;
    }
    setParent(parent, keepWorld = true) {
      for (let p = parent;p; p = p._parent) {
        if (p === this)
          throw new Error(`can't parent ${this._name} under itself`);
      }
      const t = this.transform;
      const position = t.position;
      const rotation = t.rotation;
      if (!parent)
        this._scene = this.scene;
      const siblings = this._parent ? this._parent.children : roots;
      siblings.splice(siblings.indexOf(this), 1);
      this._parent = parent;
      (parent ? parent.children : roots).push(this);
      flushTransforms();
      __iron_op("scene.parent", this.id, parent?.id ?? null);
      if (keepWorld) {
        t.position = position;
        t.rotation = rotation;
      }
      for (const f of scaleListeners)
        f(this);
    }
    get enabled() {
      return this._enabled;
    }
    set enabled(on) {
      this._enabled = on;
      flushTransforms();
      __iron_op("scene.enabled", this.id, on);
    }
    get activeInHierarchy() {
      for (let o = this;o; o = o._parent)
        if (!o._enabled)
          return false;
      return true;
    }
    get destroyed() {
      return this._destroyed;
    }
    addComponent(cls, props) {
      const instance = this._addComponentAsleep(cls, props);
      instance.onAwake?.();
      return instance;
    }
    _addComponentAsleep(cls, props) {
      const name = nameOf(cls);
      const id = nextComponent++;
      const instance = instantiate(cls, id, this);
      if (props)
        Object.assign(instance, props);
      flushTransforms();
      __iron_op("scene.addComponent", this.id, id, name);
      return instance;
    }
    getComponent(cls) {
      return this.components.find((c) => c instanceof cls);
    }
    getComponents(cls) {
      return this.components.filter((c) => c instanceof cls);
    }
    getComponentInChildren(cls) {
      return this.getComponent(cls) ?? this.children.map((c) => c.getComponentInChildren(cls)).find((c) => c);
    }
    removeComponent(component) {
      const i = this.components.indexOf(component);
      if (i < 0)
        return;
      this.components.splice(i, 1);
      flushTransforms();
      __iron_op("scene.removeComponent", component.id);
    }
    destroy() {
      if (this._destroyed)
        return;
      flushTransforms();
      __iron_op("scene.destroy", this.id);
      forget(this);
    }
    static dontDestroyOnLoad(go) {
      if (!sceneHooks)
        throw new Error("dontDestroyOnLoad needs @iron/engine's scenes.ts loaded");
      sceneHooks.dontDestroyOnLoad(go);
    }
    static instantiate(original, options = {}) {
      if (!sceneHooks)
        throw new Error("instantiate needs @iron/engine's scenes.ts loaded");
      return sceneHooks.instantiate(original, options);
    }
    findChild(name) {
      return this.children.find((c) => c._name === name);
    }
    static find(name) {
      const walk = (list) => {
        for (const o of list) {
          if (o._name === name)
            return o;
          const found = walk(o.children);
          if (found)
            return found;
        }
        return;
      };
      return walk(roots);
    }
    static get roots() {
      return roots;
    }
    static byId(id) {
      return objects.get(id);
    }
    static createPrimitive(type, options) {
      if (!primitiveFactory)
        throw new Error("createPrimitive needs @iron/engine's index (primitives.ts) loaded");
      return primitiveFactory(type, options);
    }
    static findObjectOfType(cls) {
      for (const c of instances.values())
        if (c instanceof cls)
          return c;
      return;
    }
  }
  var primitiveFactory = null;
  function setPrimitiveFactory(f) {
    primitiveFactory = f;
  }
  function forget(go) {
    const siblings = go.parent ? go.parent.children : roots;
    const i = siblings.indexOf(go);
    if (i >= 0)
      siblings.splice(i, 1);
    const stack = [go];
    while (stack.length) {
      const o = stack.pop();
      o._destroyed = true;
      objects.delete(o.id);
      stack.push(...o.children);
    }
  }
  function _forEachActiveComponent(method, f) {
    for (const c of [...instances.values()]) {
      if (typeof c[method] !== "function")
        continue;
      if (!c.gameObject?.activeInHierarchy)
        continue;
      try {
        f(c);
      } catch (e) {
        console.error(`${c.gameObject.name}: ${method}: ${e}`);
      }
    }
  }
  globalThis.__iron_registered = () => [...starting];
  globalThis.__iron_create = (className, id, objectId, objectName) => {
    const cls = classes.get(className);
    if (!cls)
      throw new Error(`unknown component class '${className}'`);
    const object = objectId === undefined || objectId === null ? GameObject._adopt(-id - 1, className) : objects.get(objectId) ?? GameObject._adopt(objectId, objectName ?? className);
    instantiate(cls, id, object).onAwake?.();
  };
  var HOOKS = ["onStart", "onUpdate", "onLateUpdate", "onFixedUpdate", "onPreRender", "onDestroy"];
  globalThis.__iron_hooks = (id) => {
    const instance = instances.get(id);
    if (!instance)
      return [...HOOKS];
    return HOOKS.filter((h) => typeof instance[h] === "function" && !(h === "onStart" && restored.has(id)));
  };
  globalThis.__iron_dispatch = (id, method, ...args) => {
    const instance = instances.get(id);
    if (!instance)
      throw new Error(`no component with id ${id}`);
    const fn = instance[method];
    if (typeof fn === "function")
      return fn.apply(instance, args);
    return;
  };
  globalThis.__iron_run = (method, ids, ...args) => {
    globalThis.__iron_run_at = -1;
    for (let i = 0;i < ids.length; i++) {
      const instance = instances.get(ids[i]);
      try {
        if (!instance)
          throw new Error(`no component with id ${ids[i]}`);
        const fn = instance[method];
        if (typeof fn === "function")
          fn.apply(instance, args);
      } catch (e) {
        globalThis.__iron_run_at = i;
        throw e;
      }
    }
  };
  globalThis.__iron_destroy = (id) => {
    const instance = instances.get(id);
    instances.delete(id);
    if (!instance)
      return;
    const list = instance.gameObject.components;
    const i = list.indexOf(instance);
    if (i >= 0)
      list.splice(i, 1);
  };
  var pendingTransforms = new Map;
  function flushTransforms() {
    if (pendingTransforms.size === 0)
      return;
    const flat = [];
    for (const [id, t] of pendingTransforms) {
      if (t.gameObject.destroyed)
        continue;
      const p = t.localPosition;
      const q = t.localRotation;
      const sc = t.localScale;
      flat.push(id, p.x, p.y, p.z, q.x, q.y, q.z, q.w, sc.x, sc.y, sc.z);
    }
    pendingTransforms.clear();
    if (flat.length)
      __iron_op("scene.locals", flat);
  }
  globalThis.__iron_flush = flushTransforms;
  globalThis.__iron_scene = (moved) => {
    for (let i = 0;i + 10 < moved.length; i += 11) {
      const go = objects.get(moved[i]);
      if (!go)
        continue;
      go.transform._set(new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]), new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]), new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]));
    }
  };

  // ../../js/engine/src/hot.ts
  var loading = false;
  var undo = [];
  var afterLoad = [];
  function _loadingGame() {
    return loading;
  }
  globalThis.__iron_hot_begin = () => {
    const old = undo;
    undo = [];
    for (const f of old) {
      try {
        f();
      } catch (e) {
        console.error(`reload: ${e}`);
      }
    }
    loading = true;
  };
  globalThis.__iron_hot_end = () => {
    loading = false;
    const stale = _patchClasses();
    if (stale.length) {
      console.warn(`reload: ${stale.join(", ")} ${stale.length === 1 ? "isn't" : "aren't"} registered, so ` + `${stale.length === 1 ? "its" : "their"} components keep the old code (register({ ${stale.join(", ")} }, { spawn: false }))`);
    }
    for (const f of afterLoad) {
      try {
        f();
      } catch (e) {
        console.error(`reload: ${e}`);
      }
    }
    return stale;
  };

  // ../../js/engine/src/property.ts
  var PROPERTIES = Symbol("iron.properties");
  var metadataKey = Symbol.metadata ?? Symbol.for("Symbol.metadata");
  function property(_value, context) {
    const meta = context.metadata;
    if (!Object.hasOwn(meta, PROPERTIES))
      meta[PROPERTIES] = [...meta[PROPERTIES] ?? []];
    const list = meta[PROPERTIES];
    const name = String(context.name);
    if (!list.includes(name))
      list.push(name);
  }
  function propertiesOf(cls) {
    const meta = cls[metadataKey];
    return meta?.[PROPERTIES] ?? [];
  }

  // ../../js/engine/src/clock.ts
  var time = 0;
  var unscaledTime = 0;
  var deltaTime = 0;
  var unscaledDeltaTime = 0;
  var frameCount = 0;
  var timeScale = 1;
  var startedAt = Date.now();
  var tickListeners = [];
  var Time = {
    get time() {
      return time;
    },
    get unscaledTime() {
      return unscaledTime;
    },
    get deltaTime() {
      return deltaTime;
    },
    get unscaledDeltaTime() {
      return unscaledDeltaTime;
    },
    get fixedDeltaTime() {
      return 1 / 60;
    },
    get frameCount() {
      return frameCount;
    },
    get timeScale() {
      return timeScale;
    },
    set timeScale(s) {
      timeScale = Number.isFinite(s) && s >= 0 ? s : 1;
    },
    get realtimeSinceStartup() {
      return (Date.now() - startedAt) / 1000;
    }
  };
  function onTick(fn) {
    tickListeners.push(fn);
  }
  function _timeState() {
    return { time, unscaledTime, frameCount, timeScale };
  }
  function _restoreTime(state) {
    const s = state;
    if (!s)
      return;
    time = s.time ?? time;
    unscaledTime = s.unscaledTime ?? unscaledTime;
    frameCount = s.frameCount ?? frameCount;
    timeScale = s.timeScale ?? timeScale;
  }
  globalThis.__iron_tick = (dt) => {
    unscaledDeltaTime = dt;
    deltaTime = dt * timeScale;
    unscaledTime += dt;
    time += deltaTime;
    frameCount++;
    for (const fn of tickListeners)
      fn();
    return deltaTime;
  };
  var pending = [];
  onTick(() => {
    const now = pending;
    pending = [];
    for (const fn of now)
      fn();
  });

  // ../../js/engine/src/physics.ts
  var vec2 = (v) => v ? [v.x, v.y, v.z] : null;
  var quat2 = (q) => q ? [q.x, q.y, q.z, q.w] : null;
  var nextBody = 1;
  var nextCollider = 1;
  var nextQuery = 1;
  var bodies = new Map;
  var colliders = new Map;
  var queries = new Map;

  class PhysicsCollider {
    id;
    body;
    sensor;
    constructor(body, options) {
      this.id = nextCollider++;
      this.body = body;
      this.sensor = !!options.sensor;
      const s = options.shape;
      let shape;
      let dims;
      let indices = null;
      if ("box" in s)
        [shape, dims] = ["box", [s.box.x, s.box.y, s.box.z]];
      else if ("ball" in s)
        [shape, dims] = ["ball", [s.ball]];
      else if ("capsule" in s)
        [shape, dims] = ["capsule", [s.capsule.halfHeight, s.capsule.radius]];
      else if ("cylinder" in s)
        [shape, dims] = ["cylinder", [s.cylinder.halfHeight, s.cylinder.radius]];
      else if ("convex" in s)
        [shape, dims] = ["convex", s.convex.flatMap((p) => [p.x, p.y, p.z])];
      else if ("renderMesh" in s) {
        const r = s.renderMesh;
        [shape, dims] = ["renderMesh", [r.mesh, r.convex ? 1 : 0, r.scale.x, r.scale.y, r.scale.z]];
      } else {
        [shape, dims] = ["mesh", s.mesh.points.flatMap((p) => [p.x, p.y, p.z])];
        indices = s.mesh.indices;
      }
      colliders.set(this.id, this);
      __iron_op("physics.collider", this.id, body?.id ?? null, shape, dims, vec2(options.position), quat2(options.rotation), [
        options.friction ?? 0.5,
        options.restitution ?? 0,
        options.density ?? 1,
        this.sensor,
        true,
        indices
      ]);
    }
    setFriction(friction) {
      __iron_op("physics.colliderSet", this.id, "friction", friction);
    }
    setRestitution(restitution) {
      __iron_op("physics.colliderSet", this.id, "restitution", restitution);
    }
    setSensor(sensor) {
      this.sensor = sensor;
      __iron_op("physics.colliderSet", this.id, "sensor", sensor);
    }
    setOffset(position, rotation) {
      __iron_op("physics.colliderSet", this.id, "offset", vec2(position), quat2(rotation));
    }
    remove() {
      if (!colliders.delete(this.id))
        return;
      __iron_op("physics.removeCollider", this.id);
    }
  }

  class PhysicsBody {
    id;
    kind;
    position;
    rotation;
    velocity;
    angularVelocity = Vector3.zero;
    grounded = false;
    onCollisionStart;
    onCollisionEnd;
    constructor(options = {}) {
      this.id = nextBody++;
      this.kind = options.kind ?? "dynamic";
      this.position = Vector3.from(options.position ?? Vector3.zero);
      this.rotation = Quaternion.from(options.rotation ?? Quaternion.identity);
      this.velocity = Vector3.from(options.velocity ?? Vector3.zero);
      bodies.set(this.id, this);
      __iron_op("physics.body", this.id, this.kind, vec2(this.position), quat2(this.rotation), vec2(this.velocity), [
        options.gravityScale ?? 1,
        options.linearDamping ?? 0,
        options.angularDamping ?? 0,
        !!options.ccd,
        !!options.lockRotations
      ]);
    }
    addCollider(options) {
      return new PhysicsCollider(this, options);
    }
    setPosition(p) {
      this.position = Vector3.from(p);
      __iron_op("physics.set", this.id, "position", vec2(p));
    }
    setRotation(q) {
      this.rotation = Quaternion.from(q);
      __iron_op("physics.set", this.id, "rotation", quat2(q));
    }
    setVelocity(v) {
      this.velocity = Vector3.from(v);
      __iron_op("physics.set", this.id, "velocity", vec2(v));
    }
    setAngularVelocity(v) {
      __iron_op("physics.set", this.id, "angularVelocity", vec2(v));
    }
    moveTo(p) {
      __iron_op("physics.set", this.id, "target", vec2(p));
    }
    applyImpulse(impulse) {
      __iron_op("physics.force", this.id, "impulse", vec2(impulse));
    }
    applyTorqueImpulse(impulse) {
      __iron_op("physics.force", this.id, "torqueImpulse", vec2(impulse));
    }
    addForce(force) {
      __iron_op("physics.force", this.id, "force", vec2(force));
    }
    addTorque(torque) {
      __iron_op("physics.force", this.id, "torque", vec2(torque));
    }
    clearForces() {
      __iron_op("physics.clearForces", this.id);
    }
    applyImpulseAtPoint(impulse, point) {
      __iron_op("physics.force", this.id, "impulseAt", vec2(impulse), vec2(point));
    }
    setKind(kind) {
      this.kind = kind;
      __iron_op("physics.set", this.id, "kind", kind);
    }
    setMass(kg) {
      __iron_op("physics.set", this.id, "mass", kg);
    }
    setGravityScale(scale) {
      __iron_op("physics.set", this.id, "gravityScale", scale);
    }
    setDamping(linear, angular) {
      __iron_op("physics.set", this.id, "linearDamping", linear);
      __iron_op("physics.set", this.id, "angularDamping", angular);
    }
    setCcd(on) {
      __iron_op("physics.set", this.id, "ccd", on);
    }
    setAllowedAxes(translation, rotation) {
      __iron_op("physics.set", this.id, "constraints", [...translation, ...rotation]);
    }
    sleep() {
      __iron_op("physics.set", this.id, "sleep");
    }
    wakeUp() {
      __iron_op("physics.set", this.id, "wake");
    }
    attachTo(objectId) {
      __iron_op("physics.attach", this.id, objectId);
    }
    setEnabled(enabled) {
      __iron_op("physics.set", this.id, "enabled", enabled);
    }
    makeCharacter(options = {}) {
      __iron_op("physics.character", this.id, options.maxSlope ?? 45, options.snapToGround ?? null, options.stepHeight ?? null);
    }
    move(delta) {
      __iron_op("physics.move", this.id, vec2(delta));
    }
    remove() {
      if (!bodies.delete(this.id))
        return;
      for (const [id, c] of colliders)
        if (c.body === this)
          colliders.delete(id);
      __iron_op("physics.removeBody", this.id);
    }
  }
  var nextJoint = 1;

  class PhysicsJoint {
    kind;
    id;
    constructor(kind, a, b, anchorA, anchorB, params = []) {
      this.kind = kind;
      this.id = nextJoint++;
      __iron_op("physics.joint", this.id, kind, a.id, b?.id ?? null, vec2(anchorA), vec2(anchorB), params);
    }
    remove() {
      __iron_op("physics.removeJoint", this.id);
    }
  }
  var physics = {
    createJoint(kind, a, b, anchorA, anchorB, params) {
      return new PhysicsJoint(kind, a, b, anchorA, anchorB, params);
    },
    createBody(options) {
      return new PhysicsBody(options);
    },
    addCollider(options) {
      return new PhysicsCollider(null, options);
    },
    setGravity(g) {
      __iron_op("physics.gravity", vec2(g));
    },
    raycast(origin, direction, maxDistance = 1000) {
      const id = nextQuery++;
      const len = Math.hypot(direction.x, direction.y, direction.z) || 1;
      const dir = { x: direction.x / len, y: direction.y / len, z: direction.z / len };
      return new Promise((resolve) => {
        queries.set(id, { origin: { ...origin }, dir, resolve });
        __iron_op("physics.raycast", id, vec2(origin), vec2(dir), maxDistance);
      });
    },
    body(id) {
      return bodies.get(id);
    }
  };
  var contactListeners = [];
  function report(fn, event) {
    if (!fn)
      return;
    try {
      fn(event);
    } catch (e) {
      const err = e instanceof Error ? e : new Error(String(e));
      console.error(`[physics] error in collision handler: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
    }
  }
  globalThis.__iron_physics = (moved, contacts, rays, characters) => {
    for (let i = 0;i + 13 < moved.length; i += 14) {
      const b = bodies.get(moved[i]);
      if (!b)
        continue;
      b.position = new Vector3(moved[i + 1], moved[i + 2], moved[i + 3]);
      b.rotation = new Quaternion(moved[i + 4], moved[i + 5], moved[i + 6], moved[i + 7]);
      b.velocity = new Vector3(moved[i + 8], moved[i + 9], moved[i + 10]);
      b.angularVelocity = new Vector3(moved[i + 11], moved[i + 12], moved[i + 13]);
    }
    for (let i = 0;i + 1 < characters.length; i += 2) {
      const b = bodies.get(characters[i]);
      if (b)
        b.grounded = characters[i + 1] === 1;
    }
    for (let i = 0;i + 4 < contacts.length; i += 5) {
      const a = colliders.get(contacts[i + 1]);
      const b = colliders.get(contacts[i + 2]);
      if (!a || !b)
        continue;
      const started = contacts[i] === 1;
      const sensor = contacts[i + 3] === 1;
      const speed = contacts[i + 4];
      for (const listener of contactListeners) {
        try {
          listener(a, b, started, sensor, speed);
        } catch (e) {
          const err = e instanceof Error ? e : new Error(String(e));
          console.error(`[physics] error in collision callback: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
        }
      }
      for (const [self, other] of [
        [a, b],
        [b, a]
      ]) {
        const body = self.body;
        if (!body)
          continue;
        report(started ? body.onCollisionStart : body.onCollisionEnd, { collider: self, other, sensor, speed });
      }
    }
    for (let i = 0;i + 5 < rays.length; i += 6) {
      const q = queries.get(rays[i]);
      if (!q)
        continue;
      queries.delete(rays[i]);
      const collider = colliders.get(rays[i + 1]);
      if (!collider) {
        q.resolve(null);
        continue;
      }
      const distance = rays[i + 2];
      q.resolve({
        collider,
        distance,
        point: new Vector3(q.origin.x + q.dir.x * distance, q.origin.y + q.dir.y * distance, q.origin.z + q.dir.z * distance),
        normal: new Vector3(rays[i + 3], rays[i + 4], rays[i + 5])
      });
    }
  };

  // ../../js/engine/src/screen.ts
  var width = 1280;
  var height = 720;
  var scale = 1;
  var display = () => globalThis.__iron_info?.display ?? {};
  var mode = () => display().mode ?? "Windowed";
  var isFullScreen = (m) => m === "FullScreenWindow" || m === "ExclusiveFullScreen";
  var windowSize = () => ({
    width: display().windowWidth ?? display().width ?? 1280,
    height: display().windowHeight ?? display().height ?? 720
  });
  function setResolution(w, h, m = mode(), refreshRate = 0) {
    const chosen = typeof m === "boolean" ? m ? "FullScreenWindow" : "Windowed" : m;
    __iron_op("screen.set", chosen, Math.round(w), Math.round(h), refreshRate);
  }
  var Screen = {
    get width() {
      return width;
    },
    get height() {
      return height;
    },
    get scale() {
      return scale;
    },
    get aspect() {
      return width / Math.max(1, height);
    },
    get fullScreenMode() {
      return mode();
    },
    set fullScreenMode(m) {
      __iron_op("screen.set", m, 0, 0, 0);
    },
    get fullScreen() {
      return isFullScreen(mode());
    },
    set fullScreen(on) {
      if (on === isFullScreen(mode()))
        return;
      __iron_op("screen.set", on ? "FullScreenWindow" : "Windowed", 0, 0, 0);
    },
    setResolution,
    get windowSize() {
      return windowSize();
    },
    get resolutions() {
      return [...display().resolutions ?? []];
    },
    get currentResolution() {
      return display().currentResolution ?? { width: 0, height: 0, refreshRate: 0 };
    },
    get supportedFullScreenModes() {
      return [...display().supportedModes ?? ["ExclusiveFullScreen", "FullScreenWindow", "MaximizedWindow", "Windowed"]];
    },
    onChange(fn) {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    }
  };
  var listeners = new Set;
  globalThis.__iron_screen = (w, h, s) => {
    width = w;
    height = h;
    scale = s;
    for (const fn of listeners) {
      try {
        fn();
      } catch (e) {
        console.error(`Screen.onChange listener failed: ${e instanceof Error ? e.stack ?? e.message : e}`);
      }
    }
  };

  // ../../js/engine/src/rendering.ts
  var rgba = (c) => [c.r, c.g, c.b, c.a];
  var cameras = [];
  var _init, _base, _dec, _dec2, _dec3, _dec4, _dec5, _dec6, _dec7, _dec8, _dec9, _dec10, _dec11, _dec12, _dec13, _dec14, _dec15;

  class Camera extends (_base = Component) {
    static {
      _init = __decoratorStart(_base);
      __decorateElement(_init, 2, "fieldOfView", _dec, this);
      __decorateElement(_init, 2, "nearClipPlane", _dec2, this);
      __decorateElement(_init, 2, "farClipPlane", _dec3, this);
      __decorateElement(_init, 2, "backgroundColor", _dec4, this);
      __decorateElement(_init, 2, "clearFlags", _dec5, this);
      __decorateElement(_init, 2, "depth", _dec6, this);
      __decorateElement(_init, 2, "orthographic", _dec7, this);
      __decorateElement(_init, 2, "orthographicSize", _dec8, this);
      __decorateElement(_init, 2, "rect", _dec9, this);
      __decorateElement(_init, 2, "cullingMask", _dec10, this);
      __decorateElement(_init, 2, "layerCullDistances", _dec11, this);
      __decorateElement(_init, 2, "layerCullSpherical", _dec12, this);
      __decorateElement(_init, 2, "antialiasing", _dec13, this);
      __decorateElement(_init, 2, "targetTexture", _dec14, this);
      __decorateElement(_init, 2, "fog", _dec15, this);
      __decoratorMetadata(_init, this);
    }
    _fov = (__runInitializers(_init, 5, this), 60);
    _near = 0.1;
    _far = 1000;
    _background = new Color(0.05, 0.06, 0.09, 1);
    _depth = 0;
    _orthographic = false;
    _orthographicSize = 5;
    _rect = new Rect(0, 0, 1, 1);
    _cullingMask = 4294967295;
    _layerCullDistances = new Array(32).fill(0);
    _layerCullSpherical = false;
    _antialiasing = "none";
    _sceneGrid = false;
    _fog = true;
    _targetTexture = null;
    started = false;
    get [(_dec = [
      property
    ], "fieldOfView")]() {
      return this._fov;
    }
    set fieldOfView(v) {
      this._fov = Math.min(179, Math.max(1, v));
      this.send();
    }
    get horizontalFieldOfView() {
      return Camera.verticalToHorizontalFieldOfView(this._fov, this.aspect);
    }
    set horizontalFieldOfView(v) {
      this.fieldOfView = Camera.horizontalToVerticalFieldOfView(v, this.aspect);
    }
    static verticalToHorizontalFieldOfView(vertical, aspect) {
      const half = vertical * Math.PI / 360;
      return Math.atan(Math.tan(half) * aspect) * 360 / Math.PI;
    }
    static horizontalToVerticalFieldOfView(horizontal, aspect) {
      const half = horizontal * Math.PI / 360;
      return Math.atan(Math.tan(half) / aspect) * 360 / Math.PI;
    }
    get [(_dec2 = [
      property
    ], "nearClipPlane")]() {
      return this._near;
    }
    set nearClipPlane(v) {
      this._near = v;
      this.send();
    }
    get [(_dec3 = [
      property
    ], "farClipPlane")]() {
      return this._far;
    }
    set farClipPlane(v) {
      this._far = v;
      this.send();
    }
    get [(_dec4 = [
      property
    ], "backgroundColor")]() {
      return this._background;
    }
    set backgroundColor(c) {
      this._background = c;
      this.send();
    }
    _clearFlags = "solidColor";
    get [(_dec5 = [
      property
    ], "clearFlags")]() {
      return this._clearFlags;
    }
    set clearFlags(f) {
      this._clearFlags = f;
      this.send();
    }
    get [(_dec6 = [
      property
    ], "depth")]() {
      return this._depth;
    }
    set depth(d) {
      this._depth = d;
      this.send();
    }
    get [(_dec7 = [
      property
    ], "orthographic")]() {
      return this._orthographic;
    }
    set orthographic(on) {
      this._orthographic = on;
      this.send();
    }
    get [(_dec8 = [
      property
    ], "orthographicSize")]() {
      return this._orthographicSize;
    }
    set orthographicSize(s) {
      this._orthographicSize = s;
      this.send();
    }
    get [(_dec9 = [
      property
    ], "rect")]() {
      return this._rect;
    }
    set rect(r) {
      this._rect = r;
      this.send();
    }
    get [(_dec10 = [
      property
    ], "cullingMask")]() {
      return this._cullingMask;
    }
    set cullingMask(mask) {
      this._cullingMask = mask >>> 0;
      this.send();
    }
    get [(_dec11 = [
      property
    ], "layerCullDistances")]() {
      return [...this._layerCullDistances];
    }
    set layerCullDistances(distances) {
      this._layerCullDistances = Array.from({ length: 32 }, (_, i) => Math.max(0, distances[i] ?? 0));
      this.send();
    }
    get [(_dec12 = [
      property
    ], "layerCullSpherical")]() {
      return this._layerCullSpherical;
    }
    set layerCullSpherical(on) {
      this._layerCullSpherical = on;
      this.send();
    }
    get [(_dec13 = [
      property
    ], "antialiasing")]() {
      return this._antialiasing;
    }
    set antialiasing(mode) {
      this._antialiasing = mode;
      this.send();
    }
    get [(_dec14 = [
      property
    ], "targetTexture")]() {
      return this._targetTexture;
    }
    set targetTexture(t) {
      this._targetTexture = t;
      this.send();
    }
    get sceneGrid() {
      return this._sceneGrid;
    }
    set sceneGrid(on) {
      this._sceneGrid = on;
      this.send();
    }
    get [(_dec15 = [
      property
    ], "fog")]() {
      return this._fog;
    }
    set fog(on) {
      this._fog = on;
      this.send();
    }
    get pixelRect() {
      return new Rect(this._rect.x * Screen.width, this._rect.y * Screen.height, this._rect.width * Screen.width, this._rect.height * Screen.height);
    }
    get aspect() {
      const r = this.pixelRect;
      return r.width / Math.max(0.000001, r.height);
    }
    static get main() {
      return cameras.find((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed && !c._targetTexture && (c.gameObject.hideFlags & HideFlags.HideInHierarchy) === 0);
    }
    static get allCameras() {
      return cameras.filter((c) => c.gameObject.activeInHierarchy && !c.gameObject.destroyed).sort((a, b) => a._depth - b._depth);
    }
    get projectionMatrix() {
      if (this._orthographic) {
        const h = this._orthographicSize;
        const w = h * this.aspect;
        return Matrix4x4.orthographic(-w, w, -h, h, this._near, this._far);
      }
      return Matrix4x4.perspective(this._fov, this.aspect, this._near, this._far);
    }
    get worldToCameraMatrix() {
      return Matrix4x4.trs(this.transform.position, this.transform.rotation).inverse;
    }
    get viewProjectionMatrix() {
      return this.projectionMatrix.mul(this.worldToCameraMatrix);
    }
    static ndc(v) {
      return new Vector2(v.x * 2 - 1, 1 - v.y * 2);
    }
    screenToViewportPoint(p) {
      const r = this.pixelRect;
      return new Vector3((p.x - r.x) / r.width, (p.y - r.y) / r.height, p.z);
    }
    viewportToScreenPoint(v) {
      const r = this.pixelRect;
      return new Vector3(r.x + v.x * r.width, r.y + v.y * r.height, v.z);
    }
    viewportPointToRay(v) {
      const inv = this.viewProjectionMatrix.inverse;
      const n = Camera.ndc(v);
      const near = inv.multiplyPoint(new Vector3(n.x, n.y, 0));
      const far = inv.multiplyPoint(new Vector3(n.x, n.y, 1));
      return new Ray(near, far.sub(near));
    }
    screenPointToRay(p) {
      return this.viewportPointToRay(this.screenToViewportPoint({ x: p.x, y: p.y, z: 0 }));
    }
    worldToViewportPoint(p) {
      const n = this.viewProjectionMatrix.multiplyPoint(p);
      const depth = Vector3.from(p).sub(this.transform.position).dot(this.transform.forward);
      return new Vector3((n.x + 1) / 2, (1 - n.y) / 2, depth);
    }
    worldToScreenPoint(p) {
      return this.viewportToScreenPoint(this.worldToViewportPoint(p));
    }
    viewportToWorldPoint(v) {
      const ray = this.viewportPointToRay(v);
      const forward = this.transform.forward;
      const along = ray.direction.dot(forward);
      const start = ray.origin.sub(this.transform.position).dot(forward);
      return ray.getPoint(along > 0.000001 ? (v.z - start) / along : 0);
    }
    screenToWorldPoint(p) {
      return this.viewportToWorldPoint(this.screenToViewportPoint(p));
    }
    send() {
      if (!this.started)
        return;
      const r = this._rect;
      __iron_op("render.camera", this.id, this.gameObject.id, this._fov, this._near, this._far, rgba(this._background), this._depth, this._orthographic, this._orthographicSize, [r.x, r.y, r.width, r.height], this._clearFlags === "skybox" ? "skybox" : "color", this._cullingMask, this._layerCullDistances.some((d) => d > 0) ? this._layerCullDistances : null, this._layerCullSpherical, this._antialiasing, this._targetTexture?.id ?? null, this._sceneGrid, this._fog);
    }
    onStart() {
      this.started = true;
      cameras.push(this);
      this.send();
    }
    onDestroy() {
      cameras.splice(cameras.indexOf(this), 1);
      __iron_op("render.remove", this.id);
    }
  }
  var _init2, _base2, _dec16, _dec17;

  class PlaneRenderer extends (_base2 = Component) {
    static {
      _init2 = __decoratorStart(_base2);
      __decorateElement(_init2, 2, "size", _dec16, this);
      __decorateElement(_init2, 2, "color", _dec17, this);
      __decoratorMetadata(_init2, this);
    }
    _size = (__runInitializers(_init2, 5, this), new Vector2(1, 1));
    _color = Color.white;
    started = false;
    get [(_dec16 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(v) {
      this._size = Vector2.from(v);
      this.send();
    }
    get [(_dec17 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    send() {
      if (!this.started)
        return;
      __iron_op("render.plane", this.id, this.gameObject.id, this._size.toArray(), rgba(this._color));
    }
    onStart() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init3, _base3, _dec18, _dec19, _dec20, _dec21, _dec22;

  class CameraFollow extends (_base3 = Component) {
    static {
      _init3 = __decoratorStart(_base3);
      __decorateElement(_init3, 5, "target", _dec18, this);
      __decorateElement(_init3, 5, "offset", _dec19, this);
      __decorateElement(_init3, 5, "offsetInTargetSpace", _dec20, this);
      __decorateElement(_init3, 5, "smoothTime", _dec21, this);
      __decorateElement(_init3, 5, "lookAtTarget", _dec22, this);
      __decoratorMetadata(_init3, this);
    }
    [(_dec18 = [
      property
    ], "target")] = __runInitializers(_init3, 8, this, null);
    [(_dec19 = [
      property
    ], "offset")] = (__runInitializers(_init3, 11, this), __runInitializers(_init3, 12, this, new Vector3(0, 3, 6)));
    [(_dec20 = [
      property
    ], "offsetInTargetSpace")] = (__runInitializers(_init3, 15, this), __runInitializers(_init3, 16, this, false));
    [(_dec21 = [
      property
    ], "smoothTime")] = (__runInitializers(_init3, 19, this), __runInitializers(_init3, 20, this, 0.25));
    [(_dec22 = [
      property
    ], "lookAtTarget")] = (__runInitializers(_init3, 23, this), __runInitializers(_init3, 24, this, true));
    velocity = (__runInitializers(_init3, 27, this), [0, 0, 0]);
    onLateUpdate(dt) {
      const t = this.target?.transform;
      if (!t || this.target?.destroyed)
        return;
      const offset = this.offsetInTargetSpace ? t.rotation.rotate(this.offset) : this.offset;
      const goal = t.position.add(offset);
      const at = this.transform.position;
      if (this.smoothTime <= 0) {
        this.transform.position = goal;
      } else {
        const axis = (i, a, b) => {
          const [v, vel] = Mathf.smoothDamp(a, b, this.velocity[i], this.smoothTime, Math.max(dt, 0.0001));
          this.velocity[i] = vel;
          return v;
        };
        this.transform.position = new Vector3(axis(0, at.x, goal.x), axis(1, at.y, goal.y), axis(2, at.z, goal.z));
      }
      if (this.lookAtTarget)
        this.transform.lookAt(t.position);
    }
  }
  var _init4, _base4, _dec23, _dec24, _dec25, _dec26, _dec27, _dec28, _dec29, _dec30;

  class CameraOrbit extends (_base4 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init4, 39, this);
    }
    static {
      _init4 = __decoratorStart(_base4);
      __decorateElement(_init4, 5, "target", _dec23, this);
      __decorateElement(_init4, 5, "center", _dec24, this);
      __decorateElement(_init4, 5, "distance", _dec25, this);
      __decorateElement(_init4, 5, "yaw", _dec26, this);
      __decorateElement(_init4, 5, "pitch", _dec27, this);
      __decorateElement(_init4, 5, "minPitch", _dec28, this);
      __decorateElement(_init4, 5, "maxPitch", _dec29, this);
      __decorateElement(_init4, 5, "autoRotate", _dec30, this);
      __decoratorMetadata(_init4, this);
    }
    [(_dec23 = [
      property
    ], "target")] = __runInitializers(_init4, 8, this, null);
    [(_dec24 = [
      property
    ], "center")] = (__runInitializers(_init4, 11, this), __runInitializers(_init4, 12, this, Vector3.zero));
    [(_dec25 = [
      property
    ], "distance")] = (__runInitializers(_init4, 15, this), __runInitializers(_init4, 16, this, 8));
    [(_dec26 = [
      property
    ], "yaw")] = (__runInitializers(_init4, 19, this), __runInitializers(_init4, 20, this, 0));
    [(_dec27 = [
      property
    ], "pitch")] = (__runInitializers(_init4, 23, this), __runInitializers(_init4, 24, this, 20));
    [(_dec28 = [
      property
    ], "minPitch")] = (__runInitializers(_init4, 27, this), __runInitializers(_init4, 28, this, -80));
    [(_dec29 = [
      property
    ], "maxPitch")] = (__runInitializers(_init4, 31, this), __runInitializers(_init4, 32, this, 80));
    [(_dec30 = [
      property
    ], "autoRotate")] = (__runInitializers(_init4, 35, this), __runInitializers(_init4, 36, this, 0));
    onLateUpdate(dt) {
      this.yaw = Mathf.repeat(this.yaw + this.autoRotate * dt, 360);
      this.pitch = Mathf.clamp(this.pitch, this.minPitch, this.maxPitch);
      const center = this.target && !this.target.destroyed ? this.target.transform.position : this.center;
      const rotation = Quaternion.euler(-this.pitch, this.yaw, 0);
      this.transform.position = center.add(rotation.rotate(Vector3.back).scale(this.distance));
      this.transform.lookAt(center);
    }
  }
  var _init5, _base5, _dec31, _dec32, _dec33, _dec34, _dec35, _dec36;

  class WorldPanel extends (_base5 = Component) {
    static {
      _init5 = __decoratorStart(_base5);
      __decorateElement(_init5, 2, "width", _dec31, this);
      __decorateElement(_init5, 2, "height", _dec32, this);
      __decorateElement(_init5, 2, "pixelsPerMeter", _dec33, this);
      __decorateElement(_init5, 2, "resolution", _dec34, this);
      __decorateElement(_init5, 2, "tint", _dec35, this);
      __decorateElement(_init5, 2, "interactive", _dec36, this);
      __decoratorMetadata(_init5, this);
    }
    _width = (__runInitializers(_init5, 5, this), 800);
    _height = 600;
    _pixelsPerMeter = 500;
    _resolution = 2;
    _tint = Color.white;
    _interactive = true;
    started = false;
    get [(_dec31 = [
      property
    ], "width")]() {
      return this._width;
    }
    set width(v) {
      this._width = v;
      this.send();
    }
    get [(_dec32 = [
      property
    ], "height")]() {
      return this._height;
    }
    set height(v) {
      this._height = v;
      this.send();
    }
    get [(_dec33 = [
      property
    ], "pixelsPerMeter")]() {
      return this._pixelsPerMeter;
    }
    set pixelsPerMeter(v) {
      this._pixelsPerMeter = v;
      this.send();
    }
    get [(_dec34 = [
      property
    ], "resolution")]() {
      return this._resolution;
    }
    set resolution(v) {
      this._resolution = v;
      this.send();
    }
    get [(_dec35 = [
      property
    ], "tint")]() {
      return this._tint;
    }
    set tint(c) {
      this._tint = c;
      this.send();
    }
    get [(_dec36 = [
      property
    ], "interactive")]() {
      return this._interactive;
    }
    set interactive(on) {
      this._interactive = on;
      this.send();
    }
    get worldSize() {
      return new Vector2(this._width / this._pixelsPerMeter, this._height / this._pixelsPerMeter);
    }
    send() {
      if (!this.started)
        return;
      __iron_op("render.panel", this.id, this.gameObject.id, [this._width, this._height], this._pixelsPerMeter, this._resolution, rgba(this._tint), this._interactive);
    }
    onStart() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var EXTRAS = {
    normalMap: null,
    normalScale: 1,
    metallicGlossMap: null,
    specularColor: null,
    occlusionMap: null,
    occlusionStrength: 1,
    emissionMap: null,
    detailAlbedoMap: null,
    detailAlbedoMapScale: Vector2.one,
    detailAlbedoMapOffset: Vector2.zero,
    renderingMode: "opaque",
    alphaCutoff: 0.5,
    doubleSided: false,
    gltfPacking: false
  };

  class Material {
    color;
    emission;
    smoothness;
    unlit;
    mainTexture;
    mainTextureScale;
    mainTextureOffset;
    metallic;
    normalMap = EXTRAS.normalMap;
    normalScale = EXTRAS.normalScale;
    metallicGlossMap = EXTRAS.metallicGlossMap;
    specularColor = EXTRAS.specularColor;
    occlusionMap = EXTRAS.occlusionMap;
    occlusionStrength = EXTRAS.occlusionStrength;
    emissionMap = EXTRAS.emissionMap;
    detailAlbedoMap = EXTRAS.detailAlbedoMap;
    detailAlbedoMapScale = EXTRAS.detailAlbedoMapScale;
    detailAlbedoMapOffset = EXTRAS.detailAlbedoMapOffset;
    renderingMode = EXTRAS.renderingMode;
    alphaCutoff = EXTRAS.alphaCutoff;
    doubleSided = EXTRAS.doubleSided;
    gltfPacking = EXTRAS.gltfPacking;
    constructor(color = Color.white, emission = Color.black, smoothness = 0.5, unlit = false, mainTexture = null, mainTextureScale = Vector2.one, mainTextureOffset = Vector2.zero, metallic = 0) {
      this.color = color;
      this.emission = emission;
      this.smoothness = smoothness;
      this.unlit = unlit;
      this.mainTexture = mainTexture;
      this.mainTextureScale = mainTextureScale;
      this.mainTextureOffset = mainTextureOffset;
      this.metallic = metallic;
    }
    static create(o = {}) {
      return new Material().with(o);
    }
    with(o) {
      const v2 = (v, d) => v ? new Vector2(v.x, v.y) : d;
      const m = new Material(o.color ?? this.color, o.emission ?? this.emission, o.smoothness ?? this.smoothness, o.unlit ?? this.unlit, o.mainTexture === undefined ? this.mainTexture : o.mainTexture, v2(o.mainTextureScale, this.mainTextureScale), v2(o.mainTextureOffset, this.mainTextureOffset), o.metallic ?? this.metallic);
      const out = m;
      for (const key of Object.keys(EXTRAS)) {
        const given = o[key];
        out[key] = given === undefined ? this[key] : given;
      }
      out.detailAlbedoMapScale = v2(o.detailAlbedoMapScale, this.detailAlbedoMapScale);
      out.detailAlbedoMapOffset = v2(o.detailAlbedoMapOffset, this.detailAlbedoMapOffset);
      return m;
    }
    withColor(color) {
      return this.with({ color });
    }
    withTexture(texture, scale = Vector2.one, offset = Vector2.zero) {
      return this.with({ mainTexture: texture, mainTextureScale: scale, mainTextureOffset: offset });
    }
    _extras() {
      const s = this.specularColor;
      return [
        this.normalMap?.id ?? null,
        this.normalScale,
        this.metallicGlossMap?.id ?? null,
        this.gltfPacking,
        this.occlusionMap?.id ?? null,
        this.occlusionStrength,
        this.emissionMap?.id ?? null,
        this.detailAlbedoMap?.id ?? null,
        [this.detailAlbedoMapScale.x, this.detailAlbedoMapScale.y, this.detailAlbedoMapOffset.x, this.detailAlbedoMapOffset.y],
        s ? [s.r, s.g, s.b] : null,
        this.renderingMode,
        this.alphaCutoff,
        this.doubleSided
      ];
    }
  }
  var nextMesh = 1;

  class Mesh {
    id;
    _data = null;
    bindposes = [];
    constructor(id) {
      this.id = id;
    }
    static _engine(id) {
      return new Mesh(id);
    }
    static _reserve() {
      return new Mesh(nextMesh++);
    }
    static create(data) {
      const mesh = new Mesh(nextMesh++);
      mesh.set(data);
      return mesh;
    }
    set(data) {
      const joints = (data.boneWeights ?? []).flatMap((b) => [...b.indices]);
      const weights = (data.boneWeights ?? []).flatMap((b) => {
        const sum = b.weights.reduce((s, w) => s + w, 0);
        return sum > 0 ? b.weights.map((w) => w / sum) : [1, 0, 0, 0];
      });
      this._data = {
        vertices: data.vertices.flatMap((v) => [v.x, v.y, v.z]),
        triangles: [...data.triangles],
        normals: (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]),
        uv: (data.uv ?? []).flatMap((v) => [v.x, v.y]),
        ...joints.length ? { joints, weights } : {}
      };
      if (data.bindposes)
        this.bindposes = [...data.bindposes];
      __iron_op("render.mesh", this.id, data.vertices.flatMap((v) => [v.x, v.y, v.z]), (data.normals ?? []).flatMap((v) => [v.x, v.y, v.z]), (data.uv ?? []).flatMap((v) => [v.x, 1 - v.y]), [...data.triangles], ...joints.length ? [joints, weights] : []);
    }
    destroy() {
      __iron_op("render.mesh", this.id, null);
    }
  }
  var _init6, _base6, _dec37, _dec38, _dec39;

  class MeshRenderer extends (_base6 = Component) {
    static {
      _init6 = __decoratorStart(_base6);
      __decorateElement(_init6, 2, "shadowCastingMode", _dec37, this);
      __decorateElement(_init6, 2, "mesh", _dec38, this);
      __decorateElement(_init6, 2, "material", _dec39, this);
      __decoratorMetadata(_init6, this);
    }
    _mesh = (__runInitializers(_init6, 5, this), "cube");
    _material = new Material;
    _shadowCastingMode = "on";
    started = false;
    get [(_dec37 = [
      property
    ], "shadowCastingMode")]() {
      return this._shadowCastingMode;
    }
    set shadowCastingMode(m) {
      this._shadowCastingMode = m;
      this.send();
    }
    get [(_dec38 = [
      property
    ], "mesh")]() {
      return this._mesh;
    }
    set mesh(m) {
      this._mesh = m;
      this.send();
    }
    get [(_dec39 = [
      property
    ], "material")]() {
      return this._material;
    }
    set material(m) {
      this._material = m;
      this.send();
    }
    _skinArgs() {
      return [];
    }
    send() {
      if (!this.started)
        return;
      const m = this._material;
      __iron_op("render.meshRenderer", this.id, this.gameObject.id, typeof this._mesh === "string" ? this._mesh : this._mesh.id, rgba(m.color), [m.emission.r, m.emission.g, m.emission.b], m.smoothness, m.unlit, m.mainTexture?.id ?? null, [m.mainTextureScale.x, m.mainTextureScale.y, m.mainTextureOffset.x, m.mainTextureOffset.y], this._shadowCastingMode !== "off", m.metallic, m._extras(), ...this._skinArgs());
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init7, _base7, _dec40, _dec41;

  class SkinnedMeshRenderer extends (_base7 = MeshRenderer) {
    constructor() {
      super(...arguments);
      __runInitializers(_init7, 11, this);
    }
    static {
      _init7 = __decoratorStart(_base7);
      __decorateElement(_init7, 2, "bones", _dec41, this);
      __decorateElement(_init7, 5, "rootBone", _dec40, this);
      __decoratorMetadata(_init7, this);
    }
    _bones = (__runInitializers(_init7, 5, this), []);
    [(_dec40 = [
      property
    ], "rootBone")] = __runInitializers(_init7, 8, this, null);
    get [(_dec41 = [
      property
    ], "bones")]() {
      return this._bones;
    }
    set bones(b) {
      this._bones = [...b];
      this.send();
    }
    _skinArgs() {
      const mesh = this.mesh;
      const bindposes = typeof mesh === "string" ? [] : mesh.bindposes;
      const count = Math.min(this._bones.length, bindposes.length);
      return [this._bones.slice(0, count).map((b) => b.id), bindposes.slice(0, count).flatMap((m) => [...m.m])];
    }
  }
  var _init8, _base8, _dec42, _dec43, _dec44, _dec45, _dec46, _dec47, _dec48, _dec49, _dec50, _dec51, _dec52, _dec53;

  class Light extends (_base8 = Component) {
    static {
      _init8 = __decoratorStart(_base8);
      __decorateElement(_init8, 2, "type", _dec42, this);
      __decorateElement(_init8, 2, "color", _dec43, this);
      __decorateElement(_init8, 2, "intensity", _dec44, this);
      __decorateElement(_init8, 2, "range", _dec45, this);
      __decorateElement(_init8, 2, "spotAngle", _dec46, this);
      __decorateElement(_init8, 2, "innerSpotAngle", _dec47, this);
      __decorateElement(_init8, 2, "haloSize", _dec48, this);
      __decorateElement(_init8, 2, "beamIntensity", _dec49, this);
      __decorateElement(_init8, 2, "shadows", _dec50, this);
      __decorateElement(_init8, 2, "shadowStrength", _dec51, this);
      __decorateElement(_init8, 2, "cullingMask", _dec52, this);
      __decorateElement(_init8, 2, "volumetricIntensity", _dec53, this);
      __decoratorMetadata(_init8, this);
    }
    _type = (__runInitializers(_init8, 5, this), "directional");
    _color = Color.white;
    _intensity = 1;
    _range = 10;
    _spotAngle = 30;
    _innerSpotAngle = 21.8;
    _haloSize = 0;
    _beamIntensity = 0;
    _shadows = "none";
    _shadowStrength = 1;
    _cullingMask = 4294967295;
    _volumetricIntensity = 1;
    started = false;
    get [(_dec42 = [
      property
    ], "type")]() {
      return this._type;
    }
    set type(t) {
      this._type = t;
      this.send();
    }
    get [(_dec43 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    get [(_dec44 = [
      property
    ], "intensity")]() {
      return this._intensity;
    }
    set intensity(i) {
      this._intensity = i;
      this.send();
    }
    get [(_dec45 = [
      property
    ], "range")]() {
      return this._range;
    }
    set range(r) {
      this._range = r;
      this.send();
    }
    get [(_dec46 = [
      property
    ], "spotAngle")]() {
      return this._spotAngle;
    }
    set spotAngle(a) {
      this._spotAngle = a;
      this.send();
    }
    get [(_dec47 = [
      property
    ], "innerSpotAngle")]() {
      return this._innerSpotAngle;
    }
    set innerSpotAngle(a) {
      this._innerSpotAngle = a;
      this.send();
    }
    get [(_dec48 = [
      property
    ], "haloSize")]() {
      return this._haloSize;
    }
    set haloSize(s) {
      this._haloSize = s;
      this.send();
    }
    get [(_dec49 = [
      property
    ], "beamIntensity")]() {
      return this._beamIntensity;
    }
    set beamIntensity(b) {
      this._beamIntensity = b;
      this.send();
    }
    get [(_dec50 = [
      property
    ], "shadows")]() {
      return this._shadows;
    }
    set shadows(s) {
      this._shadows = s;
      this.send();
    }
    get [(_dec51 = [
      property
    ], "shadowStrength")]() {
      return this._shadowStrength;
    }
    set shadowStrength(s) {
      this._shadowStrength = s;
      this.send();
    }
    get [(_dec52 = [
      property
    ], "cullingMask")]() {
      return this._cullingMask;
    }
    set cullingMask(mask) {
      this._cullingMask = mask >>> 0;
      this.send();
    }
    get [(_dec53 = [
      property
    ], "volumetricIntensity")]() {
      return this._volumetricIntensity;
    }
    set volumetricIntensity(v) {
      this._volumetricIntensity = Math.max(0, v);
      this.send();
    }
    send() {
      if (!this.started)
        return;
      const c = this._color;
      __iron_op("render.light", this.id, this.gameObject.id, this._type, [c.r, c.g, c.b], this._intensity, this._range, this._spotAngle, this._innerSpotAngle, this._haloSize, this._beamIntensity, this._shadows, this._shadowStrength, this._cullingMask, this._volumetricIntensity);
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var ambient = new Color(0.2, 0.2, 0.22);
  var bloom = null;
  var tonemapping = "none";
  var postExposure = 0;
  var sendPost = () => __iron_op("render.post", 0, postExposure, tonemapping);
  var fog = false;
  var fogMode = "exponentialSquared";
  var fogColor = new Color(0.5, 0.5, 0.5);
  var fogDensity = 0.01;
  var fogStart = 0;
  var fogEnd = 300;
  var heightFog = null;
  var volumetricFog = null;
  function sendFog() {
    if (!fog) {
      __iron_op("render.fog", 0, null);
      return;
    }
    const h = heightFog ? [heightFog.density, heightFog.baseHeight, heightFog.falloff] : null;
    const v = volumetricFog;
    const shafts = v ? [v.intensity ?? 1, v.anisotropy ?? 0.4, v.steps ?? 4, v.distance ?? 50] : null;
    __iron_op("render.fog", 0, fogMode, [fogColor.r, fogColor.g, fogColor.b], fogDensity, fogStart, fogEnd, h, shafts);
  }
  var skybox = null;
  var ambientMode = "skybox";
  var reflectionIntensity = 1;
  function sendSky() {
    if (!skybox) {
      __iron_op("render.sky", 0, null);
      return;
    }
    const s = skybox;
    const rgb = (c) => c ? [c.r, c.g, c.b] : null;
    const sun = s.sunDirection === null ? null : s.sunDirection ? [s.sunDirection.x, s.sunDirection.y, s.sunDirection.z] : [0.4, 0.6, 0.3];
    __iron_op("render.sky", 0, rgb(s.zenith) ?? [0.16, 0.32, 0.62], rgb(s.horizon) ?? [0.62, 0.72, 0.82], rgb(s.ground) ?? [0.22, 0.2, 0.18], sun, rgb(s.sunColor) ?? [20, 18, 15], s.sunSize ?? 1.5, reflectionIntensity, ambientMode === "skybox");
  }
  var RenderSettings = {
    get ambientLight() {
      return ambient;
    },
    set ambientLight(c) {
      ambient = c;
      __iron_op("render.ambient", 0, [c.r, c.g, c.b]);
    },
    get skybox() {
      return skybox;
    },
    set skybox(s) {
      skybox = s ? { ...s } : null;
      sendSky();
    },
    get ambientMode() {
      return ambientMode;
    },
    set ambientMode(m) {
      ambientMode = m;
      if (skybox)
        sendSky();
    },
    get reflectionIntensity() {
      return reflectionIntensity;
    },
    set reflectionIntensity(i) {
      reflectionIntensity = i;
      if (skybox)
        sendSky();
    },
    get bloom() {
      return bloom;
    },
    set bloom(b) {
      bloom = b ? { ...b } : null;
      if (b)
        __iron_op("render.bloom", 0, b.threshold, b.intensity, b.scatter);
      else
        __iron_op("render.bloom", 0, null);
    },
    get tonemapping() {
      return tonemapping;
    },
    set tonemapping(t) {
      tonemapping = t;
      sendPost();
    },
    get postExposure() {
      return postExposure;
    },
    set postExposure(ev) {
      postExposure = ev;
      sendPost();
    },
    get fog() {
      return fog;
    },
    set fog(on) {
      fog = on;
      sendFog();
    },
    get fogMode() {
      return fogMode;
    },
    set fogMode(m) {
      fogMode = m;
      if (fog)
        sendFog();
    },
    get fogColor() {
      return fogColor;
    },
    set fogColor(c) {
      fogColor = c;
      if (fog)
        sendFog();
    },
    get fogDensity() {
      return fogDensity;
    },
    set fogDensity(d) {
      fogDensity = Math.max(0, d);
      if (fog)
        sendFog();
    },
    get fogStartDistance() {
      return fogStart;
    },
    set fogStartDistance(d) {
      fogStart = d;
      if (fog)
        sendFog();
    },
    get fogEndDistance() {
      return fogEnd;
    },
    set fogEndDistance(d) {
      fogEnd = d;
      if (fog)
        sendFog();
    },
    get heightFog() {
      return heightFog;
    },
    set heightFog(h) {
      heightFog = h ? { ...h } : null;
      if (fog)
        sendFog();
    },
    get volumetricFog() {
      return volumetricFog;
    },
    set volumetricFog(v) {
      volumetricFog = v ? { ...v } : null;
      if (fog)
        sendFog();
    }
  };
  var _init9, _base9, _dec54, _dec55;

  class Mirror extends (_base9 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init9, 15, this);
    }
    static {
      _init9 = __decoratorStart(_base9);
      __decorateElement(_init9, 5, "normal", _dec54, this);
      __decorateElement(_init9, 5, "cullingMask", _dec55, this);
      __decoratorMetadata(_init9, this);
    }
    [(_dec54 = [
      property
    ], "normal")] = __runInitializers(_init9, 8, this, null);
    [(_dec55 = [
      property
    ], "cullingMask")] = (__runInitializers(_init9, 11, this), __runInitializers(_init9, 12, this, 4294967295));
    send() {
      const mesh = this.getComponent(MeshRenderer)?.mesh;
      const n = this.normal ?? (mesh === "quad" ? Vector3.back : Vector3.up);
      __iron_op("render.mirror", this.id, this.gameObject.id, [n.x, n.y, n.z], this.cullingMask >>> 0);
    }
    onAwake() {
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init10, _base10, _dec56, _dec57, _dec58, _dec59, _dec60;

  class ReflectionProbe extends (_base10 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init10, 27, this);
    }
    static {
      _init10 = __decoratorStart(_base10);
      __decorateElement(_init10, 5, "size", _dec56, this);
      __decorateElement(_init10, 5, "resolution", _dec57, this);
      __decorateElement(_init10, 5, "boxProjection", _dec58, this);
      __decorateElement(_init10, 5, "cullingMask", _dec59, this);
      __decorateElement(_init10, 5, "refreshMode", _dec60, this);
      __decoratorMetadata(_init10, this);
    }
    [(_dec56 = [
      property
    ], "size")] = __runInitializers(_init10, 8, this, new Vector3(10, 10, 10));
    [(_dec57 = [
      property
    ], "resolution")] = (__runInitializers(_init10, 11, this), __runInitializers(_init10, 12, this, 128));
    [(_dec58 = [
      property
    ], "boxProjection")] = (__runInitializers(_init10, 15, this), __runInitializers(_init10, 16, this, false));
    [(_dec59 = [
      property
    ], "cullingMask")] = (__runInitializers(_init10, 19, this), __runInitializers(_init10, 20, this, 4294967295));
    [(_dec60 = [
      property
    ], "refreshMode")] = (__runInitializers(_init10, 23, this), __runInitializers(_init10, 24, this, "onAwake"));
    send() {
      const s = this.size;
      __iron_op("render.probe", this.id, this.gameObject.id, [s.x, s.y, s.z], this.resolution, this.boxProjection, this.refreshMode === "everyFrame", this.cullingMask >>> 0);
    }
    renderProbe() {
      __iron_op("render.probeRender", this.id);
    }
    onAwake() {
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  var _init11, _base11, _dec61, _dec62;

  class LODGroup extends (_base11 = Component) {
    static {
      _init11 = __decoratorStart(_base11);
      __decorateElement(_init11, 2, "size", _dec61, this);
      __decorateElement(_init11, 2, "lods", _dec62, this);
      __decoratorMetadata(_init11, this);
    }
    _lods = (__runInitializers(_init11, 5, this), []);
    _size = 1;
    _forced = -1;
    started = false;
    get [(_dec61 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = s;
      this.send();
    }
    get lodCount() {
      return this._lods.length;
    }
    get [(_dec62 = [
      property
    ], "lods")]() {
      return this.getLODs();
    }
    set lods(lods) {
      this.setLODs(lods);
    }
    getLODs() {
      return this._lods.map((l) => ({ ...l, renderers: [...l.renderers] }));
    }
    setLODs(lods) {
      for (let i = 1;i < lods.length; i++) {
        if (lods[i].screenRelativeTransitionHeight > lods[i - 1].screenRelativeTransitionHeight) {
          throw new Error("LOD transition heights must go down from the first level");
        }
      }
      this._lods = lods.map((l) => ({ ...l, renderers: [...l.renderers] }));
      this.send();
    }
    forceLOD(index) {
      this._forced = index;
      this.send();
    }
    send() {
      if (!this.started)
        return;
      __iron_op("render.lodGroup", this.id, this.gameObject.id, this._size, this._lods.map((l) => l.screenRelativeTransitionHeight), this._lods.map((l) => l.renderers.map((r) => r.id)), this._forced);
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }

  // ../../js/engine/src/lines.ts
  function catmullRom(a, b, c, d, t) {
    const t2 = t * t;
    const t3 = t2 * t;
    const k = (p0, p1, p2, p3) => 0.5 * (2 * p1 + (-p0 + p2) * t + (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2 + (-p0 + 3 * p1 - 3 * p2 + p3) * t3);
    return new Vector3(k(a.x, b.x, c.x, d.x), k(a.y, b.y, c.y, d.y), k(a.z, b.z, c.z, d.z));
  }
  function tubeMesh(points, radius, sides, smoothing) {
    const src = points.map((p) => Vector3.from(p));
    const path = [];
    for (let i = 0;i < src.length - 1; i++) {
      const a = src[Math.max(0, i - 1)];
      const b = src[i];
      const c = src[i + 1];
      const d = src[Math.min(src.length - 1, i + 2)];
      for (let s = 0;s <= smoothing; s++)
        path.push(catmullRom(a, b, c, d, s / (smoothing + 1)));
    }
    path.push(src[src.length - 1]);
    const n = path.length;
    const tangents = path.map((_, i) => path[Math.min(n - 1, i + 1)].sub(path[Math.max(0, i - 1)]).normalized);
    let normal = Math.abs(tangents[0].y) < 0.9 ? Vector3.up : Vector3.right;
    normal = normal.sub(tangents[0].scale(normal.dot(tangents[0]))).normalized;
    const vertices = [];
    const normals = [];
    const uv = [];
    let along = 0;
    for (let i = 0;i < n; i++) {
      const t = tangents[i];
      if (i > 0) {
        along += path[i].distance(path[i - 1]);
        const fixed = normal.sub(t.scale(normal.dot(t)));
        if (fixed.magnitude > 0.000001)
          normal = fixed.normalized;
      }
      const binormal = t.cross(normal);
      for (let s = 0;s <= sides; s++) {
        const a = s / sides * Math.PI * 2;
        const dir = normal.scale(Math.cos(a)).add(binormal.scale(Math.sin(a)));
        vertices.push(path[i].add(dir.scale(radius)));
        normals.push(dir);
        uv.push({ x: s / sides, y: along });
      }
    }
    const triangles = [];
    const ring = sides + 1;
    for (let i = 0;i < n - 1; i++) {
      for (let s = 0;s < sides; s++) {
        const a = i * ring + s;
        const b = a + ring;
        triangles.push(a, a + 1, b, a + 1, b + 1, b);
      }
    }
    return { vertices, normals, uv, triangles };
  }
  var _init12, _base12, _dec63, _dec64, _dec65, _dec66, _dec67;

  class LineRenderer extends (_base12 = Component) {
    static {
      _init12 = __decoratorStart(_base12);
      __decorateElement(_init12, 5, "positions", _dec63, this);
      __decorateElement(_init12, 5, "width", _dec64, this);
      __decorateElement(_init12, 5, "sides", _dec65, this);
      __decorateElement(_init12, 5, "smoothing", _dec66, this);
      __decorateElement(_init12, 5, "material", _dec67, this);
      __decoratorMetadata(_init12, this);
    }
    [(_dec63 = [
      property
    ], "positions")] = __runInitializers(_init12, 8, this, []);
    [(_dec64 = [
      property
    ], "width")] = (__runInitializers(_init12, 11, this), __runInitializers(_init12, 12, this, 0.05));
    [(_dec65 = [
      property
    ], "sides")] = (__runInitializers(_init12, 15, this), __runInitializers(_init12, 16, this, 8));
    [(_dec66 = [
      property
    ], "smoothing")] = (__runInitializers(_init12, 19, this), __runInitializers(_init12, 20, this, 3));
    [(_dec67 = [
      property
    ], "material")] = (__runInitializers(_init12, 23, this), __runInitializers(_init12, 24, this, new Material(Color.white, Color.black, 0.3)));
    mesh = (__runInitializers(_init12, 27, this), null);
    renderer = null;
    drawn = "";
    onAwake() {
      this.mesh = Mesh.create({ vertices: [Vector3.zero, Vector3.zero, Vector3.zero], triangles: [0, 1, 2] });
      this.renderer = this.gameObject.addComponent(MeshRenderer, { mesh: this.mesh, material: this.material });
      this.redraw();
    }
    onPreRender() {
      this.redraw();
    }
    redraw() {
      if (!this.mesh)
        return;
      if (this.renderer && this.renderer.material !== this.material)
        this.renderer.material = this.material;
      const key = `${this.width},${this.sides},${this.smoothing},${this.positions.map((p) => `${p.x},${p.y},${p.z}`).join(";")}`;
      if (key === this.drawn || this.positions.length < 2)
        return;
      this.drawn = key;
      const local = this.positions.map((p) => this.transform.inverseTransformPoint(p));
      this.mesh.set(tubeMesh(local, this.width / 2, Math.max(3, Math.round(this.sides)), Math.max(0, Math.round(this.smoothing))));
    }
    onDestroy() {
      this.mesh?.destroy();
      this.mesh = null;
    }
  }
  var _init13, _base13, _dec68, _dec69, _dec70, _dec71, _dec72, _dec73, _dec74;

  class LineRenderer2D extends (_base13 = Component) {
    static {
      _init13 = __decoratorStart(_base13);
      __decorateElement(_init13, 2, "points", _dec68, this);
      __decorateElement(_init13, 2, "width", _dec69, this);
      __decorateElement(_init13, 2, "color", _dec70, this);
      __decorateElement(_init13, 2, "loop", _dec71, this);
      __decorateElement(_init13, 2, "fillColor", _dec72, this);
      __decorateElement(_init13, 2, "fillRule", _dec73, this);
      __decorateElement(_init13, 2, "sortingOrder", _dec74, this);
      __decoratorMetadata(_init13, this);
    }
    _points = (__runInitializers(_init13, 5, this), []);
    _width = 0.05;
    _color = Color.white;
    _loop = false;
    _fillColor = null;
    _fillRule = "evenodd";
    _sortingOrder = 0;
    started = false;
    dirty = true;
    get [(_dec68 = [
      property
    ], "points")]() {
      return this._points;
    }
    set points(p) {
      this._points = p;
      this.dirty = true;
    }
    get [(_dec69 = [
      property
    ], "width")]() {
      return this._width;
    }
    set width(w) {
      this._width = Math.max(0, w);
      this.dirty = true;
    }
    get [(_dec70 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.dirty = true;
    }
    get [(_dec71 = [
      property
    ], "loop")]() {
      return this._loop;
    }
    set loop(on) {
      this._loop = on;
      this.dirty = true;
    }
    get [(_dec72 = [
      property
    ], "fillColor")]() {
      return this._fillColor;
    }
    set fillColor(c) {
      this._fillColor = c;
      this.dirty = true;
    }
    get [(_dec73 = [
      property
    ], "fillRule")]() {
      return this._fillRule;
    }
    set fillRule(r) {
      this._fillRule = r;
      this.dirty = true;
    }
    get [(_dec74 = [
      property
    ], "sortingOrder")]() {
      return this._sortingOrder;
    }
    set sortingOrder(o) {
      this._sortingOrder = Math.trunc(o);
      this.dirty = true;
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onPreRender() {
      if (this.dirty)
        this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
    send() {
      if (!this.started)
        return;
      this.dirty = false;
      const points = this._points;
      const flat = new Array(points.length * 2);
      for (let i = 0;i < points.length; i++) {
        const p = points[i];
        flat[i * 2] = p.x;
        flat[i * 2 + 1] = p.y;
      }
      const c = this._color;
      const f = this._fillColor;
      __iron_op("render.stroke", this.id, this.gameObject.id, this._width, [c.r, c.g, c.b, c.a], this._loop, flat, f ? [f.r, f.g, f.b, f.a] : null, this._fillRule, this._sortingOrder);
    }
  }

  // ../../js/engine/src/physics-components.ts
  var RigidbodyConstraints = {
    None: 0,
    FreezePositionX: 2,
    FreezePositionY: 4,
    FreezePositionZ: 8,
    FreezeRotationX: 16,
    FreezeRotationY: 32,
    FreezeRotationZ: 64,
    FreezePosition: 14,
    FreezeRotation: 112,
    FreezeAll: 126
  };

  class PhysicsMaterial {
    dynamicFriction;
    bounciness;
    constructor(dynamicFriction = 0.6, bounciness = 0) {
      this.dynamicFriction = dynamicFriction;
      this.bounciness = bounciness;
    }
  }
  var objectPhysics = new Map;
  var colliderOwners = new Map;
  function _objectPhysics(go) {
    return objectPhysics.get(go);
  }
  function _physicsFor(go) {
    return physicsFor(go);
  }
  function _releasePhysics(go) {
    releasePhysics(go);
  }
  function physicsFor(go) {
    let p = objectPhysics.get(go);
    if (!p) {
      const t = go.transform;
      const body = physics.createBody({ kind: "fixed", position: t.position, rotation: t.rotation });
      body.attachTo(go.id);
      p = { body, rigidbody: null, colliders: new Set };
      objectPhysics.set(go, p);
    }
    return p;
  }
  function releasePhysics(go) {
    const p = objectPhysics.get(go);
    if (!p || p.rigidbody || p.colliders.size > 0)
      return;
    p.body.remove();
    objectPhysics.delete(go);
  }
  var _init14, _base14, _dec75, _dec76, _dec77;

  class Collider extends (_base14 = Component) {
    static {
      _init14 = __decoratorStart(_base14);
      __decorateElement(_init14, 2, "center", _dec75, this);
      __decorateElement(_init14, 2, "isTrigger", _dec76, this);
      __decorateElement(_init14, 2, "material", _dec77, this);
      __decoratorMetadata(_init14, this);
    }
    _center = (__runInitializers(_init14, 5, this), Vector3.zero);
    _isTrigger = false;
    _material = new PhysicsMaterial;
    builtScale = null;
    handle = null;
    get [(_dec75 = [
      property
    ], "center")]() {
      return this._center;
    }
    set center(c) {
      this._center = Vector3.from(c);
      this.handle?.setOffset(this._center.mul(this.gameObject.transform.lossyScale));
    }
    get [(_dec76 = [
      property
    ], "isTrigger")]() {
      return this._isTrigger;
    }
    set isTrigger(on) {
      this._isTrigger = on;
      this.handle?.setSensor(on);
    }
    get [(_dec77 = [
      property
    ], "material")]() {
      return this._material;
    }
    set material(m) {
      this._material = m;
      this.handle?.setFriction(m.dynamicFriction);
      this.handle?.setRestitution(m.bounciness);
    }
    get attachedRigidbody() {
      const rb = objectPhysics.get(this.gameObject)?.rigidbody;
      return rb instanceof Rigidbody ? rb : null;
    }
    get _is2D() {
      return false;
    }
    shapeRotation() {
      return;
    }
    rebuild() {
      if (!this.handle)
        return;
      this.handle.remove();
      colliderOwners.delete(this.handle.id);
      this.create();
    }
    _rescale() {
      if (this.handle && this.builtScale && !this.builtScale.equals(this.gameObject.transform.lossyScale)) {
        this.rebuild();
      }
    }
    create() {
      const p = physicsFor(this.gameObject);
      const scale = this.gameObject.transform.lossyScale;
      this.builtScale = scale;
      this.handle = p.body.addCollider({
        shape: this.shape(scale),
        position: this._center.mul(scale),
        rotation: this.shapeRotation(),
        sensor: this._isTrigger,
        friction: this._material.dynamicFriction,
        restitution: this._material.bounciness
      });
      colliderOwners.set(this.handle.id, this);
      p.colliders.add(this);
      p.rigidbody?._sendMass();
    }
    onAwake() {
      this.create();
    }
    onDestroy() {
      const p = objectPhysics.get(this.gameObject);
      p?.colliders.delete(this);
      if (this.handle) {
        colliderOwners.delete(this.handle.id);
        this.handle.remove();
        this.handle = null;
      }
      p?.rigidbody?._sendMass();
      releasePhysics(this.gameObject);
    }
  }
  var _init15, _base15, _dec78;

  class BoxCollider extends (_base15 = Collider) {
    static {
      _init15 = __decoratorStart(_base15);
      __decorateElement(_init15, 2, "size", _dec78, this);
      __decoratorMetadata(_init15, this);
    }
    _size = (__runInitializers(_init15, 5, this), Vector3.one);
    get [(_dec78 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = Vector3.from(s);
      this.rebuild();
    }
    shape(scale) {
      const h = this._size.mul(scale).scale(0.5);
      return { box: { x: Math.abs(h.x), y: Math.abs(h.y), z: Math.abs(h.z) } };
    }
  }
  var _init16, _base16, _dec79;

  class SphereCollider extends (_base16 = Collider) {
    static {
      _init16 = __decoratorStart(_base16);
      __decorateElement(_init16, 2, "radius", _dec79, this);
      __decoratorMetadata(_init16, this);
    }
    _radius = (__runInitializers(_init16, 5, this), 0.5);
    get [(_dec79 = [
      property
    ], "radius")]() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    shape(scale) {
      return { ball: this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.y), Math.abs(scale.z)) };
    }
  }
  var _init17, _base17, _dec80, _dec81;

  class CapsuleCollider extends (_base17 = Collider) {
    static {
      _init17 = __decoratorStart(_base17);
      __decorateElement(_init17, 2, "radius", _dec80, this);
      __decorateElement(_init17, 2, "height", _dec81, this);
      __decoratorMetadata(_init17, this);
    }
    _radius = (__runInitializers(_init17, 5, this), 0.5);
    _height = 2;
    get [(_dec80 = [
      property
    ], "radius")]() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    get [(_dec81 = [
      property
    ], "height")]() {
      return this._height;
    }
    set height(h) {
      this._height = h;
      this.rebuild();
    }
    shape(scale) {
      const radius = this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.z));
      const halfHeight = Math.max(0, this._height * Math.abs(scale.y) / 2 - radius);
      return { capsule: { halfHeight, radius } };
    }
  }
  var _init18, _base18, _dec82, _dec83, _dec84, _dec85;

  class MeshCollider extends (_base18 = Collider) {
    constructor() {
      super(...arguments);
      __runInitializers(_init18, 23, this);
    }
    static {
      _init18 = __decoratorStart(_base18);
      __decorateElement(_init18, 5, "sharedMesh", _dec82, this);
      __decorateElement(_init18, 5, "points", _dec83, this);
      __decorateElement(_init18, 5, "indices", _dec84, this);
      __decorateElement(_init18, 5, "convex", _dec85, this);
      __decoratorMetadata(_init18, this);
    }
    [(_dec82 = [
      property
    ], "sharedMesh")] = __runInitializers(_init18, 8, this, null);
    [(_dec83 = [
      property
    ], "points")] = (__runInitializers(_init18, 11, this), __runInitializers(_init18, 12, this, []));
    [(_dec84 = [
      property
    ], "indices")] = (__runInitializers(_init18, 15, this), __runInitializers(_init18, 16, this, []));
    [(_dec85 = [
      property
    ], "convex")] = (__runInitializers(_init18, 19, this), __runInitializers(_init18, 20, this, false));
    shape(scale) {
      if (this.sharedMesh) {
        return { renderMesh: { mesh: this.sharedMesh.id, convex: this.convex, scale } };
      }
      const pts = this.points.map((p) => Vector3.from(p).mul(scale));
      return this.convex ? { convex: pts } : { mesh: { points: pts, indices: this.indices } };
    }
    setMesh(points, indices, convex = this.convex) {
      this.points = points;
      this.indices = indices;
      this.convex = convex;
      this.rebuild();
    }
  }
  var _init19, _base19, _dec86, _dec87, _dec88, _dec89, _dec90, _dec91, _dec92;

  class Rigidbody extends (_base19 = Component) {
    static {
      _init19 = __decoratorStart(_base19);
      __decorateElement(_init19, 2, "mass", _dec86, this);
      __decorateElement(_init19, 2, "drag", _dec87, this);
      __decorateElement(_init19, 2, "angularDrag", _dec88, this);
      __decorateElement(_init19, 2, "useGravity", _dec89, this);
      __decorateElement(_init19, 2, "isKinematic", _dec90, this);
      __decorateElement(_init19, 2, "constraints", _dec91, this);
      __decorateElement(_init19, 2, "collisionDetection", _dec92, this);
      __decoratorMetadata(_init19, this);
    }
    _mass = (__runInitializers(_init19, 5, this), 1);
    _drag = 0;
    _angularDrag = 0.05;
    _useGravity = true;
    _isKinematic = false;
    _constraints = RigidbodyConstraints.None;
    _continuous = false;
    p = null;
    get body() {
      return this.p?.body ?? null;
    }
    get [(_dec86 = [
      property
    ], "mass")]() {
      return this._mass;
    }
    set mass(kg) {
      this._mass = Math.max(0.000001, kg);
      this._sendMass();
    }
    get [(_dec87 = [
      property
    ], "drag")]() {
      return this._drag;
    }
    set drag(d) {
      this._drag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec88 = [
      property
    ], "angularDrag")]() {
      return this._angularDrag;
    }
    set angularDrag(d) {
      this._angularDrag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec89 = [
      property
    ], "useGravity")]() {
      return this._useGravity;
    }
    set useGravity(on) {
      this._useGravity = on;
      this.p?.body.setGravityScale(on ? 1 : 0);
    }
    get [(_dec90 = [
      property
    ], "isKinematic")]() {
      return this._isKinematic;
    }
    set isKinematic(on) {
      this._isKinematic = on;
      this.p?.body.setKind(on ? "kinematic" : "dynamic");
    }
    get [(_dec91 = [
      property
    ], "constraints")]() {
      return this._constraints;
    }
    set constraints(flags) {
      this._constraints = flags;
      this.sendConstraints();
    }
    get [(_dec92 = [
      property
    ], "collisionDetection")]() {
      return this._continuous ? "continuous" : "discrete";
    }
    set collisionDetection(mode) {
      this._continuous = mode === "continuous";
      this.p?.body.setCcd(this._continuous);
    }
    get velocity() {
      return this.p ? Vector3.from(this.p.body.velocity) : Vector3.zero;
    }
    set velocity(v) {
      this.p?.body.setVelocity(v);
    }
    get angularVelocity() {
      return this.p ? Vector3.from(this.p.body.angularVelocity) : Vector3.zero;
    }
    set angularVelocity(v) {
      this.p?.body.setAngularVelocity(v);
    }
    addForce(force, mode = 0 /* Force */) {
      this.p?.body.applyImpulse(this.impulse(force, mode));
    }
    addTorque(torque, mode = 0 /* Force */) {
      const dt = 1 / 60;
      const t = Vector3.from(torque);
      const instant = mode === 1 /* Impulse */ || mode === 2 /* VelocityChange */;
      this.p?.body.applyTorqueImpulse(instant ? t : t.scale(dt));
    }
    addForceAtPosition(force, position, mode = 0 /* Force */) {
      this.p?.body.applyImpulseAtPoint(this.impulse(force, mode), position);
    }
    addExplosionForce(force, position, radius, upwards = 0, mode = 1 /* Impulse */) {
      const from = Vector3.from(position).sub(new Vector3(0, upwards, 0));
      const offset = this.transform.position.sub(from);
      const distance = offset.magnitude;
      if (distance > radius)
        return;
      const falloff = 1 - Mathf.clamp01(distance / Math.max(radius, 0.000001));
      this.addForce(offset.normalized.scale(force * falloff), mode);
    }
    movePosition(position) {
      this.transform.position = position;
    }
    moveRotation(rotation) {
      this.transform.rotation = rotation;
    }
    sleep() {
      this.p?.body.sleep();
    }
    wakeUp() {
      this.p?.body.wakeUp();
    }
    impulse(force, mode) {
      const f = Vector3.from(force);
      const dt = 1 / 60;
      switch (mode) {
        case 0 /* Force */:
          return f.scale(dt);
        case 5 /* Acceleration */:
          return f.scale(this._mass * dt);
        case 1 /* Impulse */:
          return f;
        case 2 /* VelocityChange */:
          return f.scale(this._mass);
      }
    }
    _sendMass() {
      this.p?.body.setMass(this._mass);
    }
    sendConstraints() {
      const c = this._constraints;
      const free = (flag) => (c & flag) === 0;
      this.p?.body.setAllowedAxes([free(RigidbodyConstraints.FreezePositionX), free(RigidbodyConstraints.FreezePositionY), free(RigidbodyConstraints.FreezePositionZ)], [free(RigidbodyConstraints.FreezeRotationX), free(RigidbodyConstraints.FreezeRotationY), free(RigidbodyConstraints.FreezeRotationZ)]);
    }
    onAwake() {
      const p = physicsFor(this.gameObject);
      p.rigidbody = this;
      this.p = p;
      p.body.setKind(this._isKinematic ? "kinematic" : "dynamic");
      this._sendMass();
      p.body.setDamping(this._drag, this._angularDrag);
      p.body.setGravityScale(this._useGravity ? 1 : 0);
      if (this._continuous)
        p.body.setCcd(true);
      if (this._constraints)
        this.sendConstraints();
    }
    onDestroy() {
      const p = this.p;
      if (!p)
        return;
      p.rigidbody = null;
      this.p = null;
      p.body.setKind("fixed");
      releasePhysics(this.gameObject);
    }
  }
  contactListeners.push((a, b, started, sensor, speed) => {
    const ca = colliderOwners.get(a.id);
    const cb = colliderOwners.get(b.id);
    if (!ca || !cb)
      return;
    for (const [self, other] of [
      [ca, cb],
      [cb, ca]
    ]) {
      const go = self.gameObject;
      if (go.destroyed)
        continue;
      for (const component of [...go.components]) {
        if (self._is2D) {
          if (sensor) {
            (started ? component.onTriggerEnter2D : component.onTriggerExit2D)?.call(component, other);
          } else {
            const collision = {
              gameObject: other.gameObject,
              collider: other,
              rigidbody: objectPhysics.get(other.gameObject)?.rigidbody ?? null,
              relativeSpeed: speed
            };
            (started ? component.onCollisionEnter2D : component.onCollisionExit2D)?.call(component, collision);
          }
        } else if (sensor) {
          (started ? component.onTriggerEnter : component.onTriggerExit)?.call(component, other);
        } else {
          const collision = {
            gameObject: other.gameObject,
            collider: other,
            rigidbody: other.attachedRigidbody,
            relativeSpeed: speed
          };
          (started ? component.onCollisionEnter : component.onCollisionExit)?.call(component, collision);
        }
      }
    }
  });
  scaleListeners.push((go) => {
    const visit = (o) => {
      const p = objectPhysics.get(o);
      if (p)
        for (const c of [...p.colliders])
          c._rescale();
      for (const child of o.children)
        visit(child);
    };
    visit(go);
  });
  var CollisionFlags = { None: 0, Sides: 1, Above: 2, Below: 4 };
  var _init20, _base20, _dec93, _dec94, _dec95, _dec96;

  class CharacterController extends (_base20 = Collider) {
    static {
      _init20 = __decoratorStart(_base20);
      __decorateElement(_init20, 2, "radius", _dec93, this);
      __decorateElement(_init20, 2, "height", _dec94, this);
      __decorateElement(_init20, 2, "slopeLimit", _dec95, this);
      __decorateElement(_init20, 2, "stepOffset", _dec96, this);
      __decoratorMetadata(_init20, this);
    }
    _radius = (__runInitializers(_init20, 5, this), 0.5);
    _height = 2;
    _slopeLimit = 45;
    _stepOffset = 0.3;
    fallSpeed = 0;
    last = Vector3.zero;
    _velocity = Vector3.zero;
    get [(_dec93 = [
      property
    ], "radius")]() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    get [(_dec94 = [
      property
    ], "height")]() {
      return this._height;
    }
    set height(h) {
      this._height = h;
      this.rebuild();
    }
    get [(_dec95 = [
      property
    ], "slopeLimit")]() {
      return this._slopeLimit;
    }
    set slopeLimit(d) {
      this._slopeLimit = d;
      this.configure();
    }
    get [(_dec96 = [
      property
    ], "stepOffset")]() {
      return this._stepOffset;
    }
    set stepOffset(h) {
      this._stepOffset = h;
      this.configure();
    }
    get isGrounded() {
      return objectPhysics.get(this.gameObject)?.body.grounded ?? false;
    }
    get velocity() {
      return this._velocity;
    }
    shape(scale) {
      const radius = this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.z));
      const halfHeight = Math.max(0, this._height * Math.abs(scale.y) / 2 - radius);
      return { capsule: { halfHeight, radius } };
    }
    configure() {
      const body = objectPhysics.get(this.gameObject)?.body;
      body?.makeCharacter({ maxSlope: this._slopeLimit, snapToGround: this._stepOffset, stepHeight: this._stepOffset });
    }
    onAwake() {
      this.create();
      objectPhysics.get(this.gameObject).body.setKind("kinematic");
      this.configure();
      this.last = this.transform.position;
    }
    onUpdate(dt) {
      const now = this.transform.position;
      this._velocity = dt > 0 ? now.sub(this.last).scale(1 / dt) : Vector3.zero;
      this.last = now;
    }
    move(motion) {
      objectPhysics.get(this.gameObject)?.body.move(motion);
      return this.isGrounded ? CollisionFlags.Below : CollisionFlags.None;
    }
    simpleMove(speed) {
      const dt = Time.deltaTime;
      this.fallSpeed = this.isGrounded ? 1 : this.fallSpeed + 9.81 * dt;
      this.move(new Vector3(speed.x * dt, -this.fallSpeed * dt, speed.z * dt));
      return this.isGrounded;
    }
  }
  var _init21, _base21, _dec97, _dec98, _dec99, _dec100;

  class Joint extends (_base21 = Component) {
    static {
      _init21 = __decoratorStart(_base21);
      __decorateElement(_init21, 5, "connectedBody", _dec97, this);
      __decorateElement(_init21, 5, "anchor", _dec98, this);
      __decorateElement(_init21, 5, "connectedAnchor", _dec99, this);
      __decorateElement(_init21, 5, "autoConfigureConnectedAnchor", _dec100, this);
      __decoratorMetadata(_init21, this);
    }
    [(_dec97 = [
      property
    ], "connectedBody")] = __runInitializers(_init21, 8, this, null);
    [(_dec98 = [
      property
    ], "anchor")] = (__runInitializers(_init21, 11, this), __runInitializers(_init21, 12, this, Vector3.zero));
    [(_dec99 = [
      property
    ], "connectedAnchor")] = (__runInitializers(_init21, 15, this), __runInitializers(_init21, 16, this, Vector3.zero));
    [(_dec100 = [
      property
    ], "autoConfigureConnectedAnchor")] = (__runInitializers(_init21, 19, this), __runInitializers(_init21, 20, this, true));
    handle = (__runInitializers(_init21, 23, this), null);
    params() {
      return [];
    }
    build() {
      this.handle?.remove();
      this.handle = null;
      const body = objectPhysics.get(this.gameObject)?.body;
      if (!body) {
        console.warn(`${this.constructor.name} on ${this.gameObject.name} needs a Rigidbody`);
        return;
      }
      const anchor = this.anchor.mul(this.transform.lossyScale);
      const other = this.connectedBody?.body ?? null;
      if (this.autoConfigureConnectedAnchor) {
        const world = this.transform.transformPoint(this.anchor);
        const t = this.connectedBody?.transform;
        this.connectedAnchor = t ? t.rotation.inverse().rotate(world.sub(t.position)) : world;
      }
      this.handle = physics.createJoint(this.kind(), body, other, anchor, this.connectedAnchor, this.params());
    }
    onStart() {
      this.build();
    }
    onDestroy() {
      this.handle?.remove();
      this.handle = null;
    }
  }

  class FixedJoint extends Joint {
    kind() {
      return "fixed";
    }
  }

  class CharacterJoint extends Joint {
    kind() {
      return "ball";
    }
  }
  var _init22, _base22, _dec101, _dec102, _dec103, _dec104, _dec105;

  class HingeJoint extends (_base22 = Joint) {
    constructor() {
      super(...arguments);
      __runInitializers(_init22, 27, this);
    }
    static {
      _init22 = __decoratorStart(_base22);
      __decorateElement(_init22, 5, "axis", _dec101, this);
      __decorateElement(_init22, 5, "useLimits", _dec102, this);
      __decorateElement(_init22, 5, "limits", _dec103, this);
      __decorateElement(_init22, 5, "useMotor", _dec104, this);
      __decorateElement(_init22, 5, "motor", _dec105, this);
      __decoratorMetadata(_init22, this);
    }
    [(_dec101 = [
      property
    ], "axis")] = __runInitializers(_init22, 8, this, new Vector3(1, 0, 0));
    [(_dec102 = [
      property
    ], "useLimits")] = (__runInitializers(_init22, 11, this), __runInitializers(_init22, 12, this, false));
    [(_dec103 = [
      property
    ], "limits")] = (__runInitializers(_init22, 15, this), __runInitializers(_init22, 16, this, { min: -90, max: 90 }));
    [(_dec104 = [
      property
    ], "useMotor")] = (__runInitializers(_init22, 19, this), __runInitializers(_init22, 20, this, false));
    [(_dec105 = [
      property
    ], "motor")] = (__runInitializers(_init22, 23, this), __runInitializers(_init22, 24, this, { targetVelocity: 0, force: 1 }));
    kind() {
      return "hinge";
    }
    params() {
      const a = this.axis;
      return [a.x, a.y, a.z, this.useLimits ? 1 : 0, this.limits.min, this.limits.max, this.useMotor ? 1 : 0, this.motor.targetVelocity, this.motor.force];
    }
    refresh() {
      if (this.handle)
        this.build();
    }
  }
  var _init23, _base23, _dec106, _dec107, _dec108;

  class SpringJoint extends (_base23 = Joint) {
    constructor() {
      super(...arguments);
      __runInitializers(_init23, 19, this);
    }
    static {
      _init23 = __decoratorStart(_base23);
      __decorateElement(_init23, 5, "spring", _dec106, this);
      __decorateElement(_init23, 5, "damper", _dec107, this);
      __decorateElement(_init23, 5, "restLength", _dec108, this);
      __decoratorMetadata(_init23, this);
    }
    [(_dec106 = [
      property
    ], "spring")] = __runInitializers(_init23, 8, this, 10);
    [(_dec107 = [
      property
    ], "damper")] = (__runInitializers(_init23, 11, this), __runInitializers(_init23, 12, this, 0.2));
    [(_dec108 = [
      property
    ], "restLength")] = (__runInitializers(_init23, 15, this), __runInitializers(_init23, 16, this, null));
    kind() {
      return "spring";
    }
    params() {
      let rest = this.restLength;
      if (rest === null) {
        const world = this.transform.transformPoint(this.anchor);
        const t = this.connectedBody?.transform;
        const other = t ? t.position.add(t.rotation.rotate(this.connectedAnchor)) : this.connectedAnchor;
        rest = world.distance(other);
      }
      return [rest, this.spring, this.damper];
    }
  }
  var _init24, _base24, _dec109;

  class RopeJoint extends (_base24 = Joint) {
    constructor() {
      super(...arguments);
      __runInitializers(_init24, 11, this);
    }
    static {
      _init24 = __decoratorStart(_base24);
      __decorateElement(_init24, 5, "maxDistance", _dec109, this);
      __decoratorMetadata(_init24, this);
    }
    [(_dec109 = [
      property
    ], "maxDistance")] = __runInitializers(_init24, 8, this, 2);
    kind() {
      return "rope";
    }
    params() {
      return [this.maxDistance];
    }
  }
  var _init25, _base25, _dec110, _dec111, _dec112, _dec113, _dec114, _dec115, _dec116;

  class Rope extends (_base25 = Component) {
    static {
      _init25 = __decoratorStart(_base25);
      __decorateElement(_init25, 5, "length", _dec110, this);
      __decorateElement(_init25, 5, "segments", _dec111, this);
      __decorateElement(_init25, 5, "radius", _dec112, this);
      __decorateElement(_init25, 5, "segmentMass", _dec113, this);
      __decorateElement(_init25, 5, "color", _dec114, this);
      __decorateElement(_init25, 5, "end", _dec115, this);
      __decorateElement(_init25, 5, "renderMode", _dec116, this);
      __decoratorMetadata(_init25, this);
    }
    [(_dec110 = [
      property
    ], "length")] = __runInitializers(_init25, 8, this, 3);
    [(_dec111 = [
      property
    ], "segments")] = (__runInitializers(_init25, 11, this), __runInitializers(_init25, 12, this, 12));
    [(_dec112 = [
      property
    ], "radius")] = (__runInitializers(_init25, 15, this), __runInitializers(_init25, 16, this, 0.04));
    [(_dec113 = [
      property
    ], "segmentMass")] = (__runInitializers(_init25, 19, this), __runInitializers(_init25, 20, this, 0.05));
    [(_dec114 = [
      property
    ], "color")] = (__runInitializers(_init25, 23, this), __runInitializers(_init25, 24, this, new Color(0.55, 0.42, 0.28, 1)));
    [(_dec115 = [
      property
    ], "end")] = (__runInitializers(_init25, 27, this), __runInitializers(_init25, 28, this, null));
    [(_dec116 = [
      property
    ], "renderMode")] = (__runInitializers(_init25, 31, this), __runInitializers(_init25, 32, this, "tube"));
    links = (__runInitializers(_init25, 35, this), []);
    joints = [];
    line = null;
    onStart() {
      const start = this.transform.position;
      const toEnd = this.end ? this.end.transform.position.sub(start) : Vector3.down.scale(this.length);
      const dir = toEnd.magnitude > 0.000001 ? toEnd.normalized : Vector3.down;
      const n = Math.max(1, Math.round(this.segments));
      const seg = this.length / n;
      const rotation = Quaternion.fromToRotation(Vector3.down, dir);
      const material = new Material(this.color, Color.black, 0.2);
      const own = objectPhysics.get(this.gameObject);
      let prev = own?.rigidbody ? own.body : null;
      let prevAnchor = prev ? Vector3.zero : start;
      for (let i = 0;i < n; i++) {
        const center = start.add(dir.scale(seg * (i + 0.5)));
        const link = new GameObject(`${this.gameObject.name} rope ${i}`, {
          position: center,
          rotation,
          scale: new Vector3(this.radius * 2, seg / 2, this.radius * 2)
        });
        if (this.renderMode === "segments")
          link.addComponent(MeshRenderer, { mesh: "capsule", material });
        link.addComponent(CapsuleCollider);
        const rb = link.addComponent(Rigidbody, { mass: this.segmentMass, angularDrag: 0.5, drag: 0.1 });
        const body = rb.body;
        this.joints.push(physics.createJoint("ball", body, prev, new Vector3(0, seg / 2, 0), prevAnchor));
        prev = body;
        prevAnchor = new Vector3(0, -seg / 2, 0);
        this.links.push(link);
      }
      if (this.end?.body && prev) {
        this.joints.push(physics.createJoint("ball", this.end.body, prev, Vector3.zero, prevAnchor));
      }
      if (this.renderMode === "tube") {
        this.line = new GameObject(`${this.gameObject.name} rope line`);
        this.line.addComponent(LineRenderer, { width: this.radius * 2, material, positions: this.points() });
      }
    }
    points() {
      if (this.links.length === 0)
        return [];
      const out = [this.links[0].transform.transformPoint(Vector3.up)];
      for (const l of this.links)
        out.push(l.transform.transformPoint(Vector3.down));
      return out;
    }
    onPreRender() {
      const r = this.line?.getComponent(LineRenderer);
      if (r) {
        r.positions = this.points();
        r.redraw();
      }
    }
    onDestroy() {
      for (const j of this.joints)
        j.remove();
      for (const l of this.links)
        l.destroy();
      this.line?.destroy();
      this.line = null;
      this.joints = [];
      this.links.length = 0;
    }
  }

  // ../../js/engine/src/cloth.ts
  var _init26, _base26, _dec117, _dec118, _dec119, _dec120, _dec121, _dec122, _dec123, _dec124, _dec125, _dec126, _dec127, _dec128, _dec129;

  class Cloth extends (_base26 = Component) {
    static {
      _init26 = __decoratorStart(_base26);
      __decorateElement(_init26, 5, "stretchingStiffness", _dec117, this);
      __decorateElement(_init26, 5, "bendingStiffness", _dec118, this);
      __decorateElement(_init26, 5, "damping", _dec119, this);
      __decorateElement(_init26, 5, "useGravity", _dec120, this);
      __decorateElement(_init26, 5, "externalAcceleration", _dec121, this);
      __decorateElement(_init26, 5, "randomAcceleration", _dec122, this);
      __decorateElement(_init26, 5, "friction", _dec123, this);
      __decorateElement(_init26, 5, "solverIterations", _dec124, this);
      __decorateElement(_init26, 5, "pins", _dec125, this);
      __decorateElement(_init26, 5, "pinRadius", _dec126, this);
      __decorateElement(_init26, 5, "pinBox", _dec127, this);
      __decorateElement(_init26, 5, "sphereColliders", _dec128, this);
      __decorateElement(_init26, 5, "capsuleColliders", _dec129, this);
      __decoratorMetadata(_init26, this);
    }
    [(_dec117 = [
      property
    ], "stretchingStiffness")] = __runInitializers(_init26, 8, this, 1);
    [(_dec118 = [
      property
    ], "bendingStiffness")] = (__runInitializers(_init26, 11, this), __runInitializers(_init26, 12, this, 0.3));
    [(_dec119 = [
      property
    ], "damping")] = (__runInitializers(_init26, 15, this), __runInitializers(_init26, 16, this, 0.1));
    [(_dec120 = [
      property
    ], "useGravity")] = (__runInitializers(_init26, 19, this), __runInitializers(_init26, 20, this, true));
    [(_dec121 = [
      property
    ], "externalAcceleration")] = (__runInitializers(_init26, 23, this), __runInitializers(_init26, 24, this, Vector3.zero));
    [(_dec122 = [
      property
    ], "randomAcceleration")] = (__runInitializers(_init26, 27, this), __runInitializers(_init26, 28, this, Vector3.zero));
    [(_dec123 = [
      property
    ], "friction")] = (__runInitializers(_init26, 31, this), __runInitializers(_init26, 32, this, 0.5));
    [(_dec124 = [
      property
    ], "solverIterations")] = (__runInitializers(_init26, 35, this), __runInitializers(_init26, 36, this, 8));
    [(_dec125 = [
      property
    ], "pins")] = (__runInitializers(_init26, 39, this), __runInitializers(_init26, 40, this, []));
    [(_dec126 = [
      property
    ], "pinRadius")] = (__runInitializers(_init26, 43, this), __runInitializers(_init26, 44, this, 0.05));
    [(_dec127 = [
      property
    ], "pinBox")] = (__runInitializers(_init26, 47, this), __runInitializers(_init26, 48, this, null));
    [(_dec128 = [
      property
    ], "sphereColliders")] = (__runInitializers(_init26, 51, this), __runInitializers(_init26, 52, this, []));
    [(_dec129 = [
      property
    ], "capsuleColliders")] = (__runInitializers(_init26, 55, this), __runInitializers(_init26, 56, this, []));
    renderer = (__runInitializers(_init26, 59, this), undefined);
    original;
    simulated = null;
    onStart() {
      this.rebuild();
    }
    rebuild() {
      const renderer = this.gameObject.components.find((c) => c.constructor.name === "MeshRenderer" || c.constructor.name === "SkinnedMeshRenderer");
      if (!renderer) {
        console.warn(`${this.gameObject.name}: Cloth needs a MeshRenderer on the same object`);
        return;
      }
      if (!this.simulated) {
        this.renderer = renderer;
        this.original = renderer.mesh;
        this.simulated = Mesh._reserve();
      }
      const source = this.original;
      const v = (x) => [x.x, x.y, x.z];
      const colliders = [];
      for (const go of this.sphereColliders) {
        const c = go.getComponent(SphereCollider);
        if (c)
          colliders.push(["sphere", go.id, c.radius, v(c.center)]);
      }
      for (const go of this.capsuleColliders) {
        const c = go.getComponent(CapsuleCollider);
        if (c)
          colliders.push(["capsule", go.id, c.radius, c.height, v(c.center)]);
      }
      const box = this.pinBox;
      __iron_op("cloth.cloth", this.id, this.gameObject.id, typeof source === "string" ? source : source.id, this.simulated.id, [this.stretchingStiffness, this.bendingStiffness, this.damping, this.friction, this.solverIterations], v(this.externalAcceleration), v(this.randomAcceleration), this.useGravity, this.pins.flatMap(v), this.pinRadius, box ? [...v(box.min), ...v(box.max)] : null, colliders);
      renderer.mesh = this.simulated;
    }
    onDestroy() {
      __iron_op("cloth.remove", this.id);
      if (this.renderer && this.original !== undefined && !this.renderer.gameObject.destroyed)
        this.renderer.mesh = this.original;
      this.simulated?.destroy();
    }
  }
  // ../../js/engine/src/assets.ts
  var listeners2 = new Set;
  var entries = () => globalThis.__iron_info?.assets ?? [];
  var clean = (path) => path.replace(/\\/g, "/").replace(/^\.\//, "");
  var Assets = {
    get all() {
      return entries();
    },
    guidToPath(guid) {
      return entries().find((e) => e.guid === guid)?.path;
    },
    pathToGuid(path) {
      const p = clean(path);
      return entries().find((e) => e.path === p)?.guid;
    },
    importer(path) {
      const p = clean(Assets.resolve(path));
      return entries().find((e) => e.path === p)?.importer ?? {};
    },
    resolve(src) {
      if (!src.startsWith("guid:"))
        return src;
      return Assets.guidToPath(src.slice(5)) ?? src;
    },
    onChanged(fn) {
      listeners2.add(fn);
      return () => listeners2.delete(fn);
    }
  };
  globalThis.__iron_assets_changed = (paths) => {
    for (const f of [...listeners2]) {
      try {
        f(paths);
      } catch (e) {
        console.error(`Assets.onChanged: ${e}`);
      }
    }
  };
  // ../../js/engine/src/animation.ts
  class Keyframe {
    time;
    value;
    inTangent;
    outTangent;
    constructor(time, value, inTangent = 0, outTangent = 0) {
      this.time = time;
      this.value = value;
      this.inTangent = inTangent;
      this.outTangent = outTangent;
    }
  }
  function wrap(t, length, mode) {
    if (length <= 0)
      return 0;
    if (mode === "loop")
      return (t % length + length) % length;
    if (mode === "pingPong") {
      const p = (t % (2 * length) + 2 * length) % (2 * length);
      return p > length ? 2 * length - p : p;
    }
    return Math.min(length, Math.max(0, t));
  }

  class AnimationCurve {
    keys;
    preWrapMode = "clampForever";
    postWrapMode = "clampForever";
    constructor(...keys) {
      this.keys = [...keys].sort((a, b) => a.time - b.time);
    }
    static linear(t0, v0, t1, v1) {
      const slope = t1 === t0 ? 0 : (v1 - v0) / (t1 - t0);
      return new AnimationCurve(new Keyframe(t0, v0, 0, slope), new Keyframe(t1, v1, slope, 0));
    }
    static easeInOut(t0, v0, t1, v1) {
      return new AnimationCurve(new Keyframe(t0, v0), new Keyframe(t1, v1));
    }
    static constant(t0, t1, value) {
      return new AnimationCurve(new Keyframe(t0, value), new Keyframe(t1, value));
    }
    static fromPoints(points, mode = "linear") {
      const keys = points.map(([t, v]) => new Keyframe(t, v));
      for (let i = 0;i < keys.length; i++) {
        const prev = keys[i - 1];
        const next = keys[i + 1];
        const k = keys[i];
        const slope = (a, b) => b.time === a.time ? 0 : (b.value - a.value) / (b.time - a.time);
        if (mode === "step") {
          k.inTangent = k.outTangent = Infinity;
        } else if (mode === "linear") {
          k.inTangent = prev ? slope(prev, k) : 0;
          k.outTangent = next ? slope(k, next) : 0;
        } else {
          const s = prev && next ? slope(prev, next) : prev ? slope(prev, k) : next ? slope(k, next) : 0;
          k.inTangent = k.outTangent = s;
        }
      }
      return new AnimationCurve(...keys);
    }
    addKey(time, value = 0) {
      const key = time instanceof Keyframe ? time : new Keyframe(time, value);
      let i = this.keys.findIndex((k) => k.time > key.time);
      if (i < 0)
        i = this.keys.length;
      this.keys.splice(i, 0, key);
      return i;
    }
    removeKey(index) {
      this.keys.splice(index, 1);
    }
    get length() {
      return this.keys.length;
    }
    evaluate(time) {
      const keys = this.keys;
      if (keys.length === 0)
        return 0;
      if (keys.length === 1)
        return keys[0].value;
      const first = keys[0].time;
      const last = keys[keys.length - 1].time;
      const span = last - first;
      let t = time;
      if (t < first)
        t = first + wrap(t - first, span, this.preWrapMode);
      else if (t > last)
        t = first + wrap(t - first, span, this.postWrapMode);
      let lo = 0;
      let hi = keys.length - 1;
      while (hi - lo > 1) {
        const mid = lo + hi >> 1;
        if (keys[mid].time <= t)
          lo = mid;
        else
          hi = mid;
      }
      const a = keys[lo];
      const b = keys[hi];
      const dt = b.time - a.time;
      if (dt <= 0)
        return b.value;
      if (!Number.isFinite(a.outTangent) || !Number.isFinite(b.inTangent))
        return t >= b.time ? b.value : a.value;
      const s = (t - a.time) / dt;
      const s2 = s * s;
      const s3 = s2 * s;
      return (2 * s3 - 3 * s2 + 1) * a.value + (s3 - 2 * s2 + s) * a.outTangent * dt + (-2 * s3 + 3 * s2) * b.value + (s3 - s2) * b.inTangent * dt;
    }
    _toJSON() {
      const n = (x) => Number.isFinite(x) ? x : "Infinity";
      return {
        keys: this.keys.map((k) => [k.time, k.value, n(k.inTangent), n(k.outTangent)]),
        pre: this.preWrapMode,
        post: this.postWrapMode
      };
    }
    static _fromJSON(data) {
      const d = data;
      const num = (x) => x === "Infinity" ? Infinity : Number(x);
      const curve = new AnimationCurve(...(d.keys ?? []).map((k) => new Keyframe(num(k[0]), num(k[1]), num(k[2]), num(k[3]))));
      curve.preWrapMode = d.pre ?? "clampForever";
      curve.postWrapMode = d.post ?? "clampForever";
      return curve;
    }
  }

  class AnimationClip {
    name;
    wrapMode = "once";
    frameRate = 60;
    events = [];
    _bindings = [];
    constructor(name = "clip") {
      this.name = name;
    }
    setCurve(relativePath, type, propertyName, curve) {
      const typeName = typeof type === "string" ? type : type.name;
      const i = this._bindings.findIndex((b) => b.path === relativePath && b.type === typeName && b.property === propertyName);
      if (i >= 0)
        this._bindings.splice(i, 1);
      if (curve)
        this._bindings.push({ path: relativePath, type: typeName, property: propertyName, curve });
    }
    get curves() {
      return this._bindings;
    }
    get length() {
      let end = 0;
      for (const b of this._bindings)
        end = Math.max(end, b.curve.keys.at(-1)?.time ?? 0);
      for (const e of this.events)
        end = Math.max(end, e.time);
      return end;
    }
    addEvent(event) {
      this.events.push({ ...event });
      this.events.sort((a, b) => a.time - b.time);
    }
    sampleAnimation(go, time) {
      const pose = new Pose;
      pose.add(go, this, wrap(time, this.length, this.wrapMode), 1);
      pose.apply();
    }
    toJSON() {
      return {
        name: this.name,
        wrapMode: this.wrapMode,
        frameRate: this.frameRate,
        curves: this._bindings.map((b) => ({ path: b.path, type: b.type, property: b.property, curve: b.curve._toJSON() })),
        events: this.events.map((e) => ({ ...e }))
      };
    }
    static fromJSON(data) {
      const d = data;
      const clip = new AnimationClip(d.name ?? "clip");
      clip.wrapMode = d.wrapMode ?? "once";
      clip.frameRate = d.frameRate ?? 60;
      for (const c of d.curves ?? [])
        clip.setCurve(c.path, c.type, c.property, AnimationCurve._fromJSON(c.curve));
      for (const e of d.events ?? [])
        clip.addEvent(e);
      return clip;
    }
  }
  function resolve(root, path) {
    if (path === "")
      return root;
    let go = root;
    for (const name of path.split("/")) {
      go = go?.findChild(name);
      if (!go)
        return;
    }
    return go;
  }
  function componentNamed(go, name) {
    return go.components.find((c) => c.constructor.name === name);
  }

  class Pose {
    transforms = new Map;
    props = new Map;
    add(root, clip, time, weight) {
      if (weight <= 0)
        return;
      for (const b of clip._bindings) {
        const go = resolve(root, b.path);
        if (!go)
          continue;
        const value = b.curve.evaluate(time);
        if (b.type === "Transform") {
          let t = this.transforms.get(go);
          if (!t) {
            t = { p: [0, 0, 0], r: [0, 0, 0, 0], s: [0, 0, 0], e: [0, 0, 0], w: {} };
            this.transforms.set(go, t);
          }
          const [field, axis] = b.property.split(".");
          const i = "xyzw".indexOf(axis);
          const list = field === "localPosition" ? t.p : field === "localRotation" ? t.r : field === "localScale" ? t.s : field === "localEulerAngles" ? t.e : null;
          if (!list || i < 0)
            continue;
          list[i] += value * weight;
          t.w[b.property] = (t.w[b.property] ?? 0) + weight;
          continue;
        }
        const target = b.type === "GameObject" ? go : componentNamed(go, b.type);
        if (!target)
          continue;
        const key = `${go.id}/${b.type}/${b.property}`;
        const prev = this.props.get(key);
        if (prev) {
          prev.value += value * weight;
          prev.weight += weight;
        } else {
          this.props.set(key, { target, property: b.property, value: value * weight, weight });
        }
      }
    }
    apply() {
      for (const [go, t] of this.transforms) {
        if (go.destroyed)
          continue;
        const tr = go.transform;
        const has = (f, n) => [..."xyzw".slice(0, n)].some((a) => t.w[`${f}.${a}`]);
        const avg = (f, list, current) => current.map((c, i) => {
          const w = t.w[`${f}.${"xyzw"[i]}`];
          return w ? list[i] / w : c;
        });
        const cur = tr.localPosition;
        const v3 = (a) => new Vector3(a[0], a[1], a[2]);
        const position = has("localPosition", 3) ? v3(avg("localPosition", t.p, [cur.x, cur.y, cur.z])) : cur;
        const sc = tr.localScale;
        const scale = has("localScale", 3) ? v3(avg("localScale", t.s, [sc.x, sc.y, sc.z])) : sc;
        let rotation = tr.localRotation;
        if (has("localRotation", 4)) {
          const q = avg("localRotation", t.r, [rotation.x, rotation.y, rotation.z, rotation.w]);
          rotation = new Quaternion(q[0], q[1], q[2], q[3]).normalized;
        } else if (has("localEulerAngles", 3)) {
          const e0 = tr.localEulerAngles;
          const e = avg("localEulerAngles", t.e, [e0.x, e0.y, e0.z]);
          rotation = Quaternion.euler(e[0], e[1], e[2]);
        }
        tr._setLocal(position, rotation, scale);
      }
      for (const p of this.props.values()) {
        const value = p.value / p.weight;
        if (p.target instanceof GameObject) {
          if (p.property === "active")
            p.target.enabled = value > 0.5;
          continue;
        }
        setProperty(p.target, p.property, value);
      }
    }
    addAligned(root, clip, time, weight) {
      const flips = new Map;
      for (const b of clip._bindings) {
        if (b.type !== "Transform" || !b.property.startsWith("localRotation."))
          continue;
        const go = resolve(root, b.path);
        if (!go || flips.has(go))
          continue;
        const sum = this.transforms.get(go)?.r;
        if (!sum || sum.every((v) => v === 0))
          continue;
        const q = "xyzw".split("").map((a) => clip._bindings.find((c) => c.path === b.path && c.type === "Transform" && c.property === `localRotation.${a}`)?.curve.evaluate(time) ?? 0);
        const dot = q.reduce((d, v, i) => d + v * sum[i], 0);
        flips.set(go, dot < 0 ? -1 : 1);
      }
      if ([...flips.values()].every((f) => f > 0)) {
        this.add(root, clip, time, weight);
        return;
      }
      for (const b of clip._bindings) {
        const go = resolve(root, b.path);
        const flip = go && b.type === "Transform" && b.property.startsWith("localRotation.") ? flips.get(go) ?? 1 : 1;
        const single = new AnimationClip;
        single._bindings.push(flip < 0 ? { ...b, curve: negated(b.curve) } : b);
        this.add(root, single, time, weight);
      }
    }
  }
  var negatedCache = new WeakMap;
  function negated(c) {
    let n = negatedCache.get(c);
    if (!n) {
      n = new AnimationCurve(...c.keys.map((k) => new Keyframe(k.time, -k.value, -k.inTangent, -k.outTangent)));
      n.preWrapMode = c.preWrapMode;
      n.postWrapMode = c.postWrapMode;
      negatedCache.set(c, n);
    }
    return n;
  }
  function setProperty(target, property, value) {
    const record = target;
    const dot = property.indexOf(".");
    if (dot < 0) {
      const current = record[property];
      record[property] = typeof current === "boolean" ? value > 0.5 : value;
      return;
    }
    const head = property.slice(0, dot);
    const field = property.slice(dot + 1);
    const current = record[head];
    if (current === null || typeof current !== "object")
      return;
    const copy = Object.assign(Object.create(Object.getPrototypeOf(current)), current);
    copy[field] = value;
    record[head] = copy;
  }
  function fireEvents(go, clip, from, to, length, wrapMode) {
    if (clip.events.length === 0 || to <= from)
      return;
    const passes = wrapMode === "loop" && length > 0 ? Math.floor(to / length) - Math.floor(from / length) : 0;
    const local = (t) => wrapMode === "loop" && length > 0 ? (t % length + length) % length : t;
    const a = local(from);
    const b = local(to);
    for (const e of clip.events) {
      const hit = passes === 0 ? e.time > a && e.time <= b : e.time > a || e.time <= b || passes > 1;
      if (!hit)
        continue;
      for (const c of [...go.components]) {
        const fn = c[e.functionName];
        if (typeof fn === "function")
          fn.call(c, e.parameter);
      }
    }
  }

  class AnimationState {
    name;
    clip;
    time = 0;
    speed = 1;
    weight = 0;
    enabled = false;
    wrapMode;
    _fade = 0;
    _target = 0;
    constructor(name, clip) {
      this.name = name;
      this.clip = clip;
      this.wrapMode = clip.wrapMode;
    }
    get length() {
      return this.clip.length;
    }
    get normalizedTime() {
      return this.length > 0 ? this.time / this.length : 0;
    }
    set normalizedTime(t) {
      this.time = t * this.length;
    }
  }
  var _init27, _base27, _dec130, _dec131, _dec132;

  class Animation extends (_base27 = Component) {
    static {
      _init27 = __decoratorStart(_base27);
      __decorateElement(_init27, 5, "clip", _dec130, this);
      __decorateElement(_init27, 5, "clips", _dec131, this);
      __decorateElement(_init27, 5, "playAutomatically", _dec132, this);
      __decoratorMetadata(_init27, this);
    }
    [(_dec130 = [
      property
    ], "clip")] = __runInitializers(_init27, 8, this, null);
    [(_dec131 = [
      property
    ], "clips")] = (__runInitializers(_init27, 11, this), __runInitializers(_init27, 12, this, []));
    [(_dec132 = [
      property
    ], "playAutomatically")] = (__runInitializers(_init27, 15, this), __runInitializers(_init27, 16, this, true));
    states = (__runInitializers(_init27, 19, this), new Map);
    onStart() {
      if (this.playAutomatically && this.clip)
        this.play(this.clip.name);
    }
    addClip(clip, name = clip.name) {
      if (!this.clips.includes(clip))
        this.clips.push(clip);
      this.states.set(name, new AnimationState(name, clip));
    }
    state(name) {
      if (!this.states.has(name)) {
        const clip = [...this.clips, ...this.clip ? [this.clip] : []].find((c) => c.name === name);
        if (clip)
          this.states.set(name, new AnimationState(name, clip));
      }
      return this.states.get(name);
    }
    play(name = this.clip?.name ?? "") {
      const s = this.state(name);
      if (!s)
        return false;
      for (const other of this.states.values()) {
        if (other !== s) {
          other.enabled = false;
          other.weight = 0;
        }
      }
      s.enabled = true;
      s.time = 0;
      s.weight = 1;
      s._fade = 0;
      s._target = 1;
      return true;
    }
    crossFade(name, fadeLength = 0.3) {
      const s = this.state(name);
      if (!s)
        return;
      if (!s.enabled) {
        s.enabled = true;
        s.time = 0;
      }
      const rate = 1 / Math.max(0.0001, fadeLength);
      s._target = 1;
      s._fade = rate;
      for (const other of this.states.values()) {
        if (other !== s && other.enabled) {
          other._target = 0;
          other._fade = rate;
        }
      }
    }
    stop(name) {
      for (const s of this.states.values()) {
        if (name === undefined || s.name === name) {
          s.enabled = false;
          s.weight = 0;
          s.time = 0;
        }
      }
    }
    isPlaying(name) {
      return [...this.states.values()].some((s) => s.enabled && (name === undefined || s.name === name));
    }
    _animate(dt) {
      const pose = new Pose;
      for (const s of this.states.values()) {
        if (!s.enabled)
          continue;
        if (s._fade) {
          const step = s._fade * dt;
          s.weight = s.weight < s._target ? Math.min(s._target, s.weight + step) : Math.max(s._target, s.weight - step);
          if (s.weight === s._target)
            s._fade = 0;
          if (s.weight <= 0 && s._target === 0) {
            s.enabled = false;
            continue;
          }
        }
        const before = s.time;
        s.time += dt * s.speed;
        fireEvents(this.gameObject, s.clip, before, s.time, s.length, s.wrapMode);
        if (s.wrapMode === "once" && s.time >= s.length) {
          pose.addAligned(this.gameObject, s.clip, s.length, s.weight);
          s.enabled = false;
          continue;
        }
        pose.addAligned(this.gameObject, s.clip, wrap(s.time, s.length, s.wrapMode), s.weight);
      }
      pose.apply();
    }
  }

  class AnimatorController {
    parameters = [];
    states = [];
    transitions = [];
    defaultState = null;
    constructor(init = {}) {
      Object.assign(this, init);
    }
    addParameter(name, type, defaultValue) {
      this.parameters.push({ name, type, defaultValue });
      return this;
    }
    addState(name, motion, options = {}) {
      this.states.push({ name, motion, ...options });
      return this;
    }
    addTransition(t) {
      this.transitions.push(t);
      return this;
    }
    toJSON() {
      const motion = (m) => m instanceof AnimationClip ? { clip: m.toJSON() } : { blendParameter: m.blendParameter, children: m.children.map((c) => ({ ...c, motion: c.motion.toJSON() })) };
      return {
        parameters: this.parameters,
        states: this.states.map((s) => ({ ...s, motion: motion(s.motion) })),
        transitions: this.transitions,
        defaultState: this.defaultState
      };
    }
    static fromJSON(data) {
      const d = data;
      return new AnimatorController({
        parameters: d.parameters ?? [],
        transitions: d.transitions ?? [],
        defaultState: d.defaultState ?? null,
        states: (d.states ?? []).map((s) => ({
          ...s,
          motion: s.motion.clip ? AnimationClip.fromJSON(s.motion.clip) : {
            blendParameter: s.motion.blendParameter ?? "",
            children: (s.motion.children ?? []).map((c) => ({ ...c, motion: AnimationClip.fromJSON(c.motion) }))
          }
        }))
      });
    }
  }
  var _init28, _base28, _dec133, _dec134;

  class Animator extends (_base28 = Component) {
    static {
      _init28 = __decoratorStart(_base28);
      __decorateElement(_init28, 5, "controller", _dec133, this);
      __decorateElement(_init28, 5, "speed", _dec134, this);
      __decoratorMetadata(_init28, this);
    }
    [(_dec133 = [
      property
    ], "controller")] = __runInitializers(_init28, 8, this, null);
    [(_dec134 = [
      property
    ], "speed")] = (__runInitializers(_init28, 11, this), __runInitializers(_init28, 12, this, 1));
    values = (__runInitializers(_init28, 15, this), new Map);
    current = null;
    next = null;
    ensure() {
      if (this.current || !this.controller)
        return;
      for (const p of this.controller.parameters) {
        if (!this.values.has(p.name))
          this.values.set(p.name, p.defaultValue ?? (p.type === "bool" || p.type === "trigger" ? false : 0));
      }
      const name = this.controller.defaultState ?? this.controller.states[0]?.name;
      const state = this.controller.states.find((s) => s.name === name);
      if (state)
        this.current = { state, time: 0 };
    }
    setFloat(name, value) {
      this.values.set(name, value);
    }
    getFloat(name) {
      this.ensure();
      return Number(this.values.get(name) ?? 0);
    }
    setInteger(name, value) {
      this.values.set(name, Math.trunc(value));
    }
    getInteger(name) {
      return Math.trunc(this.getFloat(name));
    }
    setBool(name, value) {
      this.values.set(name, value);
    }
    getBool(name) {
      this.ensure();
      return this.values.get(name) === true;
    }
    setTrigger(name) {
      this.values.set(name, true);
    }
    resetTrigger(name) {
      this.values.set(name, false);
    }
    play(stateName, normalizedTime = 0) {
      this.ensure();
      const state = this.controller?.states.find((s) => s.name === stateName);
      if (!state)
        return;
      this.current = { state, time: normalizedTime * this.lengthOf(state) };
      this.next = null;
    }
    crossFade(stateName, duration = 0.25, normalizedTime = 0) {
      this.ensure();
      const state = this.controller?.states.find((s) => s.name === stateName);
      if (!state || !this.current) {
        if (state)
          this.play(stateName, normalizedTime);
        return;
      }
      this.next = { state, time: normalizedTime * this.lengthOf(state), progress: 0, duration: Math.max(0.0001, duration) };
    }
    get isInTransition() {
      return this.next !== null;
    }
    getCurrentAnimatorStateInfo() {
      this.ensure();
      return this.info(this.current);
    }
    getNextAnimatorStateInfo() {
      return this.info(this.next);
    }
    info(p) {
      const length = p ? this.lengthOf(p.state) : 0;
      const name = p?.state.name ?? "";
      return { name, length, normalizedTime: p && length > 0 ? p.time / length : 0, isName: (n) => n === name };
    }
    clips(state) {
      const m = state.motion;
      if (m instanceof AnimationClip)
        return [{ clip: m, weight: 1, timeScale: 1 }];
      const children = [...m.children].sort((a, b) => a.threshold - b.threshold);
      if (children.length === 0)
        return [];
      const v = this.getFloat(m.blendParameter);
      const first = children[0];
      const last = children[children.length - 1];
      if (v <= first.threshold)
        return [{ clip: first.motion, weight: 1, timeScale: first.timeScale ?? 1 }];
      if (v >= last.threshold)
        return [{ clip: last.motion, weight: 1, timeScale: last.timeScale ?? 1 }];
      const i = children.findIndex((c) => c.threshold > v);
      const a = children[i - 1];
      const b = children[i];
      const t = (v - a.threshold) / (b.threshold - a.threshold);
      return [
        { clip: a.motion, weight: 1 - t, timeScale: a.timeScale ?? 1 },
        { clip: b.motion, weight: t, timeScale: b.timeScale ?? 1 }
      ];
    }
    lengthOf(state) {
      return this.clips(state).reduce((l, c) => l + c.weight * c.clip.length / c.timeScale, 0);
    }
    stateSpeed(state) {
      const factor = state.speedParameter ? this.getFloat(state.speedParameter) : 1;
      return (state.speed ?? 1) * factor * this.speed;
    }
    passes(c) {
      const v = this.values.get(c.parameter);
      const n = Number(v ?? 0);
      const t = c.threshold ?? 0;
      switch (c.mode) {
        case "if":
          return v === true;
        case "ifNot":
          return v !== true;
        case "greater":
          return n > t;
        case "less":
          return n < t;
        case "equals":
          return n === t;
        case "notEqual":
          return n !== t;
      }
    }
    checkTransitions() {
      if (!this.controller || !this.current || this.next)
        return;
      const from = this.current;
      const length = this.lengthOf(from.state);
      const normalized = length > 0 ? from.time / length : 1;
      for (const t of this.controller.transitions) {
        if (t.from !== from.state.name && !(t.from === "Any" && t.to !== from.state.name))
          continue;
        if (t.hasExitTime && normalized < (t.exitTime ?? 1))
          continue;
        const conditions = t.conditions ?? [];
        if (!t.hasExitTime && conditions.length === 0)
          continue;
        if (!conditions.every((c) => this.passes(c)))
          continue;
        for (const c of conditions) {
          if (this.controller.parameters.find((p) => p.name === c.parameter)?.type === "trigger")
            this.values.set(c.parameter, false);
        }
        const state = this.controller.states.find((s) => s.name === t.to);
        if (!state)
          continue;
        const duration = t.duration ?? 0.25;
        const start = (t.offset ?? 0) * this.lengthOf(state);
        if (duration <= 0)
          this.current = { state, time: start };
        else
          this.next = { state, time: start, progress: 0, duration };
        return;
      }
    }
    _animate(dt) {
      this.ensure();
      if (!this.current)
        return;
      this.checkTransitions();
      const pose = new Pose;
      const run = (p, weight) => {
        const before = p.time;
        p.time += dt * this.stateSpeed(p.state);
        const length = this.lengthOf(p.state);
        for (const c of this.clips(p.state)) {
          const t = length > 0 ? p.time / length * c.clip.length : 0;
          const t0 = length > 0 ? before / length * c.clip.length : 0;
          const mode = c.clip.wrapMode === "once" ? "clampForever" : c.clip.wrapMode;
          if (c.weight * weight >= 0.5)
            fireEvents(this.gameObject, c.clip, t0, t, c.clip.length, mode);
          pose.addAligned(this.gameObject, c.clip, wrap(t, c.clip.length, mode), c.weight * weight);
        }
      };
      if (this.next) {
        this.next.progress = Math.min(1, this.next.progress + dt / this.next.duration);
        const w = this.next.progress;
        run(this.current, 1 - w);
        run(this.next, w);
        if (w >= 1) {
          this.current = { state: this.next.state, time: this.next.time };
          this.next = null;
        }
      } else {
        run(this.current, 1);
      }
      pose.apply();
    }
  }
  globalThis.__iron_animate = (dt) => {
    const visit = (go) => {
      if (!go.activeInHierarchy)
        return;
      for (const c of go.components) {
        if (c instanceof Animation || c instanceof Animator) {
          try {
            c._animate(dt);
          } catch (e) {
            console.error(`animation on ${go.name}: ${e}`);
          }
        }
      }
      for (const child of go.children)
        visit(child);
    };
    for (const root of GameObject.roots)
      visit(root);
  };
  // ../../js/engine/src/audio.ts
  var nextSound = 1;
  var nextPlayback = 1;
  var nextTrack = 1;
  var playing = new Map;

  class Sound {
    id;
    src;
    constructor(source, options = {}) {
      const src = Assets.resolve(source);
      this.id = nextSound++;
      this.src = src;
      if (options.stream === undefined && Assets.importer(src).stream === true)
        options = { ...options, stream: true };
      __iron_op("audio.load", this.id, src, !!options.stream);
    }
    play(options = {}) {
      return new Playback(this, options);
    }
    unload() {
      __iron_op("audio.unload", this.id);
    }
  }

  class Playback {
    id;
    sound;
    playing = true;
    onEnded;
    constructor(sound, o) {
      this.id = nextPlayback++;
      this.sound = sound;
      playing.set(this.id, this);
      __iron_op("audio.play", this.id, sound.id, o.on?.id ?? null, [
        o.volume ?? 1,
        o.rate ?? 1,
        o.pan ?? 0,
        !!o.loop,
        o.fadeIn ?? 0,
        o.start ?? 0
      ]);
    }
    setVolume(volume, fade = 0) {
      __iron_op("audio.set", this.id, "volume", volume, fade);
    }
    setRate(rate, fade = 0) {
      __iron_op("audio.set", this.id, "rate", rate, fade);
    }
    setPan(pan, fade = 0) {
      __iron_op("audio.set", this.id, "pan", pan, fade);
    }
    pause(fade = 0) {
      __iron_op("audio.pause", this.id, fade);
    }
    resume(fade = 0) {
      __iron_op("audio.resume", this.id, fade);
    }
    stop(fade = 0) {
      __iron_op("audio.stop", this.id, fade);
    }
    seek(seconds) {
      __iron_op("audio.seek", this.id, seconds);
    }
  }

  class Bus {
    id;
    constructor(volume, parent) {
      this.id = nextTrack++;
      __iron_op("audio.bus", this.id, parent?.id ?? null, volume);
    }
    setVolume(volume, fade = 0) {
      __iron_op("audio.trackVolume", this.id, volume, fade);
    }
    pause(fade = 0) {
      __iron_op("audio.trackPause", this.id, fade);
    }
    resume(fade = 0) {
      __iron_op("audio.trackResume", this.id, fade);
    }
    remove() {
      __iron_op("audio.removeTrack", this.id);
    }
  }

  class Emitter {
    id;
    constructor(position, options) {
      this.id = nextTrack++;
      __iron_op("audio.emitter", this.id, [position.x, position.y, position.z], [options.minDistance ?? 1, options.maxDistance ?? 50], options.bus?.id ?? null);
    }
    setPosition(p, glide = 0) {
      __iron_op("audio.emitterPosition", this.id, [p.x, p.y, p.z], glide);
    }
    setVolume(volume, fade = 0) {
      __iron_op("audio.trackVolume", this.id, volume, fade);
    }
    remove() {
      __iron_op("audio.removeTrack", this.id);
    }
  }
  var cache = new Map;
  var audio = {
    load(src, options) {
      return new Sound(src, options);
    },
    play(src, options) {
      let sound = cache.get(src);
      if (!sound) {
        sound = new Sound(src);
        cache.set(src, sound);
      }
      return sound.play(options);
    },
    createBus(options = {}) {
      return new Bus(options.volume ?? 1, options.parent ?? null);
    },
    createEmitter(position, options = {}) {
      return new Emitter(position, options);
    },
    setListener(position, rotation = { x: 0, y: 0, z: 0, w: 1 }) {
      __iron_op("audio.listener", [position.x, position.y, position.z], [rotation.x, rotation.y, rotation.z, rotation.w]);
    },
    setMasterVolume(volume, fade = 0) {
      __iron_op("audio.trackVolume", null, volume, fade);
    }
  };
  globalThis.__iron_audio = (ended) => {
    for (const id of ended) {
      const p = playing.get(id);
      if (!p)
        continue;
      playing.delete(id);
      p.playing = false;
      try {
        p.onEnded?.();
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[audio] error in onEnded: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  };
  // ../../js/engine/src/audio-components.ts
  var clips = new Map;
  function clipOf(clip) {
    if (clip instanceof Sound)
      return clip;
    let sound = clips.get(clip);
    if (!sound) {
      sound = audio.load(clip);
      clips.set(clip, sound);
    }
    return sound;
  }
  var _init29, _base29, _dec135, _dec136, _dec137, _dec138, _dec139, _dec140, _dec141, _dec142, _dec143, _dec144;

  class AudioSource extends (_base29 = Component) {
    static {
      _init29 = __decoratorStart(_base29);
      __decorateElement(_init29, 2, "volume", _dec140, this);
      __decorateElement(_init29, 2, "pitch", _dec141, this);
      __decorateElement(_init29, 2, "panStereo", _dec142, this);
      __decorateElement(_init29, 2, "mute", _dec143, this);
      __decorateElement(_init29, 2, "loop", _dec144, this);
      __decorateElement(_init29, 5, "clip", _dec135, this);
      __decorateElement(_init29, 5, "playOnAwake", _dec136, this);
      __decorateElement(_init29, 5, "spatialBlend", _dec137, this);
      __decorateElement(_init29, 5, "minDistance", _dec138, this);
      __decorateElement(_init29, 5, "maxDistance", _dec139, this);
      __decoratorMetadata(_init29, this);
    }
    [(_dec135 = [
      property
    ], "clip")] = (__runInitializers(_init29, 5, this), __runInitializers(_init29, 8, this, null));
    [(_dec136 = [
      property
    ], "playOnAwake")] = (__runInitializers(_init29, 11, this), __runInitializers(_init29, 12, this, true));
    [(_dec137 = [
      property
    ], "spatialBlend")] = (__runInitializers(_init29, 15, this), __runInitializers(_init29, 16, this, 0));
    [(_dec138 = [
      property
    ], "minDistance")] = (__runInitializers(_init29, 19, this), __runInitializers(_init29, 20, this, 1));
    [(_dec139 = [
      property
    ], "maxDistance")] = (__runInitializers(_init29, 23, this), __runInitializers(_init29, 24, this, 50));
    outputBus = (__runInitializers(_init29, 27, this), null);
    _volume = 1;
    _pitch = 1;
    _panStereo = 0;
    _loop = false;
    _mute = false;
    main = null;
    paused = false;
    all = new Set;
    emitter = null;
    emitterKey = "";
    sentPosition = null;
    get [(_dec140 = [
      property
    ], "volume")]() {
      return this._volume;
    }
    set volume(v) {
      this._volume = v;
      this.eachPlaying((p) => p.setVolume(this.gain()));
    }
    get [(_dec141 = [
      property
    ], "pitch")]() {
      return this._pitch;
    }
    set pitch(r) {
      this._pitch = r;
      this.eachPlaying((p) => p.setRate(r));
    }
    get [(_dec142 = [
      property
    ], "panStereo")]() {
      return this._panStereo;
    }
    set panStereo(p) {
      this._panStereo = p;
      this.eachPlaying((pb) => pb.setPan(p));
    }
    get [(_dec143 = [
      property
    ], "mute")]() {
      return this._mute;
    }
    set mute(m) {
      this._mute = m;
      this.eachPlaying((p) => p.setVolume(this.gain()));
    }
    get [(_dec144 = [
      property
    ], "loop")]() {
      return this._loop;
    }
    set loop(l) {
      this._loop = l;
    }
    get isPlaying() {
      return !!this.main?.playing && !this.paused;
    }
    gain() {
      return this._mute ? 0 : this._volume;
    }
    eachPlaying(f) {
      for (const p of this.all)
        f(p);
    }
    output() {
      if (this.spatialBlend <= 0)
        return this.outputBus ?? undefined;
      const key = `${this.minDistance},${this.maxDistance},${this.outputBus?.id ?? ""}`;
      if (!this.emitter || key !== this.emitterKey) {
        this.emitter?.remove();
        this.all.clear();
        this.main = null;
        this.sentPosition = this.transform.position;
        this.emitter = audio.createEmitter(this.sentPosition, {
          minDistance: this.minDistance,
          maxDistance: this.maxDistance,
          bus: this.outputBus ?? undefined
        });
        this.emitterKey = key;
      }
      return this.emitter;
    }
    start(sound, options) {
      const p = sound.play({ ...options, on: this.output() });
      this.all.add(p);
      p.onEnded = () => this.all.delete(p);
      return p;
    }
    play() {
      if (!this.clip) {
        console.warn(`AudioSource on ${this.gameObject.name} has no clip`);
        return;
      }
      this.main?.stop();
      this.paused = false;
      this.main = this.start(clipOf(this.clip), {
        volume: this.gain(),
        rate: this._pitch,
        pan: this._panStereo,
        loop: this._loop
      });
    }
    stop() {
      this.main?.stop();
      this.main = null;
      this.paused = false;
    }
    pause() {
      if (!this.main?.playing || this.paused)
        return;
      this.main.pause();
      this.paused = true;
    }
    unPause() {
      if (!this.paused || !this.main)
        return;
      this.main.resume();
      this.paused = false;
    }
    playOneShot(clip, volumeScale = 1) {
      this.start(clipOf(clip), { volume: this.gain() * volumeScale, rate: this._pitch, pan: this._panStereo });
    }
    static playClipAtPoint(clip, position, volume = 1) {
      const emitter = audio.createEmitter(position);
      const p = clipOf(clip).play({ volume, on: emitter });
      p.onEnded = () => emitter.remove();
      return p;
    }
    onAwake() {
      if (this.playOnAwake && this.clip)
        this.play();
    }
    onPreRender() {
      if (!this.emitter)
        return;
      const p = this.transform.position;
      const s = this.sentPosition;
      if (s && p.x === s.x && p.y === s.y && p.z === s.z)
        return;
      this.emitter.setPosition(p);
      this.sentPosition = p;
    }
    onDestroy() {
      for (const p of this.all)
        p.stop();
      this.all.clear();
      this.main = null;
      this.emitter?.remove();
      this.emitter = null;
    }
  }
  var masterVolume = 1;

  class AudioListener extends Component {
    sent = null;
    static get volume() {
      return masterVolume;
    }
    static set volume(v) {
      masterVolume = v;
      audio.setMasterVolume(v);
    }
    onAwake() {
      this.send();
    }
    onPreRender() {
      this.send();
    }
    send() {
      const p = this.transform.position;
      const r = this.transform.rotation;
      const s = this.sent;
      if (s && s.x === p.x && s.y === p.y && s.z === p.z && s.rx === r.x && s.ry === r.y && s.rz === r.z && s.rw === r.w) {
        return;
      }
      audio.setListener(p, r);
      this.sent = { x: p.x, y: p.y, z: p.z, rx: r.x, ry: r.y, rz: r.z, rw: r.w };
    }
  }
  // ../../js/engine/src/physics2d.ts
  var gravity2D = new Vector2(0, -9.81);
  var Physics2D = {
    depth: 10,
    get gravity() {
      return gravity2D;
    },
    set gravity(g) {
      gravity2D = new Vector2(g.x, g.y);
      physics.setGravity({ x: g.x, y: g.y, z: 0 });
    },
    async raycast(origin, direction, distance = 1000) {
      const hit = await physics.raycast({ x: origin.x, y: origin.y, z: 0 }, { x: direction.x, y: direction.y, z: 0 }, distance);
      if (!hit)
        return null;
      return {
        collider: colliderFor(hit.collider),
        point: new Vector2(hit.point.x, hit.point.y),
        normal: new Vector2(hit.normal.x, hit.normal.y),
        distance: hit.distance
      };
    }
  };
  var colliders2D = new Map;
  function colliderFor(c) {
    return colliders2D.get(c.id) ?? null;
  }
  var _init30, _base30, _dec145;

  class Collider2D extends (_base30 = Collider) {
    constructor() {
      super(...arguments);
      __runInitializers(_init30, 5, this);
    }
    static {
      _init30 = __decoratorStart(_base30);
      __decorateElement(_init30, 2, "offset", _dec145, this);
      __decoratorMetadata(_init30, this);
    }
    get [(_dec145 = [
      property
    ], "offset")]() {
      return new Vector2(this.center.x, this.center.y);
    }
    set offset(o) {
      this.center = new Vector3(o.x, o.y, 0);
    }
    get _is2D() {
      return true;
    }
    get attachedRigidbody2D() {
      const rb = _objectPhysics(this.gameObject)?.rigidbody;
      return rb instanceof Rigidbody2D ? rb : null;
    }
    halfDepth() {
      return Physics2D.depth / 2;
    }
    slab(points, scale) {
      const d = this.halfDepth();
      const pts = [];
      for (const p of points) {
        pts.push(new Vector3(p.x * scale.x, p.y * scale.y, -d), new Vector3(p.x * scale.x, p.y * scale.y, d));
      }
      return { convex: pts };
    }
    create() {
      if (this.handle)
        colliders2D.delete(this.handle.id);
      super.create();
      if (this.handle)
        colliders2D.set(this.handle.id, this);
    }
    onDestroy() {
      if (this.handle)
        colliders2D.delete(this.handle.id);
      super.onDestroy();
    }
  }
  var _init31, _base31, _dec146;

  class BoxCollider2D extends (_base31 = Collider2D) {
    static {
      _init31 = __decoratorStart(_base31);
      __decorateElement(_init31, 2, "size", _dec146, this);
      __decoratorMetadata(_init31, this);
    }
    _size = (__runInitializers(_init31, 5, this), Vector2.one);
    get [(_dec146 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = new Vector2(s.x, s.y);
      this.rebuild();
    }
    shape(scale) {
      return {
        box: {
          x: Math.abs(this._size.x * scale.x / 2),
          y: Math.abs(this._size.y * scale.y / 2),
          z: this.halfDepth()
        }
      };
    }
  }
  var _init32, _base32, _dec147;

  class CircleCollider2D extends (_base32 = Collider2D) {
    static {
      _init32 = __decoratorStart(_base32);
      __decorateElement(_init32, 2, "radius", _dec147, this);
      __decoratorMetadata(_init32, this);
    }
    _radius = (__runInitializers(_init32, 5, this), 0.5);
    get [(_dec147 = [
      property
    ], "radius")]() {
      return this._radius;
    }
    set radius(r) {
      this._radius = r;
      this.rebuild();
    }
    shape(scale) {
      return {
        cylinder: {
          halfHeight: this.halfDepth(),
          radius: this._radius * Math.max(Math.abs(scale.x), Math.abs(scale.y))
        }
      };
    }
    shapeRotation() {
      return Quaternion.euler(90, 0, 0);
    }
  }
  var _init33, _base33, _dec148, _dec149;

  class CapsuleCollider2D extends (_base33 = Collider2D) {
    static {
      _init33 = __decoratorStart(_base33);
      __decorateElement(_init33, 2, "size", _dec148, this);
      __decorateElement(_init33, 2, "direction", _dec149, this);
      __decoratorMetadata(_init33, this);
    }
    _size = (__runInitializers(_init33, 5, this), new Vector2(1, 2));
    _direction = "vertical";
    get [(_dec148 = [
      property
    ], "size")]() {
      return this._size;
    }
    set size(s) {
      this._size = new Vector2(s.x, s.y);
      this.rebuild();
    }
    get [(_dec149 = [
      property
    ], "direction")]() {
      return this._direction;
    }
    set direction(d) {
      this._direction = d;
      this.rebuild();
    }
    shape(scale) {
      const vertical = this._direction === "vertical";
      const [w, h] = [this._size.x, this._size.y];
      const r = (vertical ? w : h) / 2;
      const half = Math.max(0, (vertical ? h : w) / 2 - r);
      const pts = [];
      for (let end = 0;end < 2; end++) {
        for (let i = 0;i <= 8; i++) {
          const a = Math.PI * (i / 8) + (end ? Math.PI : 0);
          const along = end ? -half : half;
          pts.push(vertical ? new Vector2(Math.cos(a) * r, along + Math.sin(a) * r) : new Vector2(along + Math.sin(a) * r, -Math.cos(a) * r));
        }
      }
      return this.slab(pts, scale);
    }
  }
  var _init34, _base34, _dec150;

  class PolygonCollider2D extends (_base34 = Collider2D) {
    static {
      _init34 = __decoratorStart(_base34);
      __decorateElement(_init34, 2, "points", _dec150, this);
      __decoratorMetadata(_init34, this);
    }
    _points = (__runInitializers(_init34, 5, this), [new Vector2(-0.5, -0.5), new Vector2(0.5, -0.5), new Vector2(0, 0.5)]);
    get [(_dec150 = [
      property
    ], "points")]() {
      return this._points;
    }
    set points(p) {
      this._points = p.map((v) => new Vector2(v.x, v.y));
      this.rebuild();
    }
    shape(scale) {
      return this.slab(this._points, scale);
    }
  }
  var _init35, _base35, _dec151;

  class EdgeCollider2D extends (_base35 = Collider2D) {
    static {
      _init35 = __decoratorStart(_base35);
      __decorateElement(_init35, 2, "points", _dec151, this);
      __decoratorMetadata(_init35, this);
    }
    _points = (__runInitializers(_init35, 5, this), [new Vector2(-0.5, 0), new Vector2(0.5, 0)]);
    get [(_dec151 = [
      property
    ], "points")]() {
      return this._points;
    }
    set points(p) {
      this._points = p.map((v) => new Vector2(v.x, v.y));
      this.rebuild();
    }
    shape(scale) {
      const d = this.halfDepth();
      const pts = [];
      const indices = [];
      this._points.forEach((p, i) => {
        pts.push(new Vector3(p.x * scale.x, p.y * scale.y, -d), new Vector3(p.x * scale.x, p.y * scale.y, d));
        if (i > 0) {
          const [a, b, c, e] = [(i - 1) * 2, (i - 1) * 2 + 1, i * 2, i * 2 + 1];
          indices.push(a, c, b, b, c, e);
        }
      });
      return { mesh: { points: pts, indices } };
    }
  }
  var _init36, _base36, _dec152, _dec153, _dec154, _dec155, _dec156, _dec157, _dec158;

  class Rigidbody2D extends (_base36 = Component) {
    static {
      _init36 = __decoratorStart(_base36);
      __decorateElement(_init36, 2, "mass", _dec152, this);
      __decorateElement(_init36, 2, "drag", _dec153, this);
      __decorateElement(_init36, 2, "angularDrag", _dec154, this);
      __decorateElement(_init36, 2, "gravityScale", _dec155, this);
      __decorateElement(_init36, 2, "bodyType", _dec156, this);
      __decorateElement(_init36, 2, "freezeRotation", _dec157, this);
      __decorateElement(_init36, 2, "collisionDetectionMode", _dec158, this);
      __decoratorMetadata(_init36, this);
    }
    _mass = (__runInitializers(_init36, 5, this), 1);
    _drag = 0;
    _angularDrag = 0.05;
    _gravityScale = 1;
    _bodyType = "dynamic";
    _freezeRotation = false;
    _continuous = false;
    p = null;
    get [(_dec152 = [
      property
    ], "mass")]() {
      return this._mass;
    }
    set mass(kg) {
      this._mass = Math.max(0.000001, kg);
      this._sendMass();
    }
    get [(_dec153 = [
      property
    ], "drag")]() {
      return this._drag;
    }
    set drag(d) {
      this._drag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec154 = [
      property
    ], "angularDrag")]() {
      return this._angularDrag;
    }
    set angularDrag(d) {
      this._angularDrag = d;
      this.p?.body.setDamping(this._drag, this._angularDrag);
    }
    get [(_dec155 = [
      property
    ], "gravityScale")]() {
      return this._gravityScale;
    }
    set gravityScale(s) {
      this._gravityScale = s;
      this.p?.body.setGravityScale(s);
    }
    get [(_dec156 = [
      property
    ], "bodyType")]() {
      return this._bodyType;
    }
    set bodyType(t) {
      this._bodyType = t;
      this.p?.body.setKind(t === "static" ? "fixed" : t);
    }
    get isKinematic() {
      return this._bodyType === "kinematic";
    }
    set isKinematic(on) {
      this.bodyType = on ? "kinematic" : "dynamic";
    }
    get [(_dec157 = [
      property
    ], "freezeRotation")]() {
      return this._freezeRotation;
    }
    set freezeRotation(on) {
      this._freezeRotation = on;
      this.sendAxes();
    }
    get [(_dec158 = [
      property
    ], "collisionDetectionMode")]() {
      return this._continuous ? "continuous" : "discrete";
    }
    set collisionDetectionMode(m) {
      this._continuous = m === "continuous";
      this.p?.body.setCcd(this._continuous);
    }
    get velocity() {
      const v = this.p?.body.velocity;
      return v ? new Vector2(v.x, v.y) : Vector2.zero;
    }
    set velocity(v) {
      this.p?.body.setVelocity({ x: v.x, y: v.y, z: 0 });
    }
    get angularVelocity() {
      return (this.p?.body.angularVelocity.z ?? 0) * 180 / Math.PI;
    }
    set angularVelocity(degrees) {
      this.p?.body.setAngularVelocity({ x: 0, y: 0, z: degrees * Math.PI / 180 });
    }
    get position() {
      const p = this.transform.position;
      return new Vector2(p.x, p.y);
    }
    set position(v) {
      this.transform.position = new Vector3(v.x, v.y, this.transform.position.z);
    }
    get rotation() {
      return this.transform.eulerAngles.z;
    }
    set rotation(degrees) {
      this.transform.rotation = Quaternion.euler(0, 0, degrees);
    }
    addForce(force, mode = 0 /* Force */) {
      const k = mode === 0 /* Force */ ? 1 / 60 : 1;
      this.p?.body.applyImpulse({ x: force.x * k, y: force.y * k, z: 0 });
    }
    addForceAtPosition(force, position, mode = 0 /* Force */) {
      const k = mode === 0 /* Force */ ? 1 / 60 : 1;
      this.p?.body.applyImpulseAtPoint({ x: force.x * k, y: force.y * k, z: 0 }, { x: position.x, y: position.y, z: this.transform.position.z });
    }
    addTorque(torque, mode = 0 /* Force */) {
      const k = mode === 0 /* Force */ ? 1 / 60 : 1;
      this.p?.body.applyTorqueImpulse({ x: 0, y: 0, z: torque * k });
    }
    movePosition(position) {
      this.position = position;
    }
    moveRotation(degrees) {
      this.rotation = degrees;
    }
    sleep() {
      this.p?.body.sleep();
    }
    wakeUp() {
      this.p?.body.wakeUp();
    }
    _sendMass() {
      this.p?.body.setMass(this._mass);
    }
    sendAxes() {
      this.p?.body.setAllowedAxes([true, true, false], [false, false, !this._freezeRotation]);
    }
    onAwake() {
      const p = _physicsFor(this.gameObject);
      p.rigidbody = this;
      this.p = p;
      p.body.setKind(this._bodyType === "static" ? "fixed" : this._bodyType);
      this._sendMass();
      p.body.setDamping(this._drag, this._angularDrag);
      p.body.setGravityScale(this._gravityScale);
      if (this._continuous)
        p.body.setCcd(true);
      this.sendAxes();
    }
    onDestroy() {
      const p = this.p;
      if (!p)
        return;
      p.rigidbody = null;
      this.p = null;
      p.body.setKind("fixed");
      _releasePhysics(this.gameObject);
    }
  }
  // ../../js/engine/src/particles.ts
  var systems = new Map;
  var _init37, _base37, _dec159, _dec160, _dec161, _dec162, _dec163, _dec164, _dec165, _dec166, _dec167, _dec168, _dec169, _dec170, _dec171, _dec172, _dec173, _dec174, _dec175, _dec176, _dec177, _dec178, _dec179, _dec180, _dec181, _dec182;

  class ParticleSystem extends (_base37 = Component) {
    static {
      _init37 = __decoratorStart(_base37);
      __decorateElement(_init37, 5, "duration", _dec159, this);
      __decorateElement(_init37, 5, "loop", _dec160, this);
      __decorateElement(_init37, 5, "playOnAwake", _dec161, this);
      __decorateElement(_init37, 5, "startDelay", _dec162, this);
      __decorateElement(_init37, 5, "startLifetime", _dec163, this);
      __decorateElement(_init37, 5, "startSpeed", _dec164, this);
      __decorateElement(_init37, 5, "startSize", _dec165, this);
      __decorateElement(_init37, 5, "startRotation", _dec166, this);
      __decorateElement(_init37, 5, "angularVelocity", _dec167, this);
      __decorateElement(_init37, 5, "startColor", _dec168, this);
      __decorateElement(_init37, 5, "gravityModifier", _dec169, this);
      __decorateElement(_init37, 5, "drag", _dec170, this);
      __decorateElement(_init37, 5, "simulationSpace", _dec171, this);
      __decorateElement(_init37, 5, "maxParticles", _dec172, this);
      __decorateElement(_init37, 5, "randomSeed", _dec173, this);
      __decorateElement(_init37, 5, "rateOverTime", _dec174, this);
      __decorateElement(_init37, 5, "bursts", _dec175, this);
      __decorateElement(_init37, 5, "shape", _dec176, this);
      __decorateElement(_init37, 5, "sizeOverLifetime", _dec177, this);
      __decorateElement(_init37, 5, "colorOverLifetime", _dec178, this);
      __decorateElement(_init37, 5, "texture", _dec179, this);
      __decorateElement(_init37, 5, "blendMode", _dec180, this);
      __decorateElement(_init37, 5, "textureSheet", _dec181, this);
      __decorateElement(_init37, 5, "stopAction", _dec182, this);
      __decoratorMetadata(_init37, this);
    }
    [(_dec159 = [
      property
    ], "duration")] = __runInitializers(_init37, 8, this, 5);
    [(_dec160 = [
      property
    ], "loop")] = (__runInitializers(_init37, 11, this), __runInitializers(_init37, 12, this, true));
    [(_dec161 = [
      property
    ], "playOnAwake")] = (__runInitializers(_init37, 15, this), __runInitializers(_init37, 16, this, true));
    [(_dec162 = [
      property
    ], "startDelay")] = (__runInitializers(_init37, 19, this), __runInitializers(_init37, 20, this, 0));
    [(_dec163 = [
      property
    ], "startLifetime")] = (__runInitializers(_init37, 23, this), __runInitializers(_init37, 24, this, 5));
    [(_dec164 = [
      property
    ], "startSpeed")] = (__runInitializers(_init37, 27, this), __runInitializers(_init37, 28, this, 5));
    [(_dec165 = [
      property
    ], "startSize")] = (__runInitializers(_init37, 31, this), __runInitializers(_init37, 32, this, 1));
    [(_dec166 = [
      property
    ], "startRotation")] = (__runInitializers(_init37, 35, this), __runInitializers(_init37, 36, this, 0));
    [(_dec167 = [
      property
    ], "angularVelocity")] = (__runInitializers(_init37, 39, this), __runInitializers(_init37, 40, this, 0));
    [(_dec168 = [
      property
    ], "startColor")] = (__runInitializers(_init37, 43, this), __runInitializers(_init37, 44, this, null));
    [(_dec169 = [
      property
    ], "gravityModifier")] = (__runInitializers(_init37, 47, this), __runInitializers(_init37, 48, this, 0));
    [(_dec170 = [
      property
    ], "drag")] = (__runInitializers(_init37, 51, this), __runInitializers(_init37, 52, this, 0));
    [(_dec171 = [
      property
    ], "simulationSpace")] = (__runInitializers(_init37, 55, this), __runInitializers(_init37, 56, this, "local"));
    [(_dec172 = [
      property
    ], "maxParticles")] = (__runInitializers(_init37, 59, this), __runInitializers(_init37, 60, this, 1000));
    [(_dec173 = [
      property
    ], "randomSeed")] = (__runInitializers(_init37, 63, this), __runInitializers(_init37, 64, this, 1));
    [(_dec174 = [
      property
    ], "rateOverTime")] = (__runInitializers(_init37, 67, this), __runInitializers(_init37, 68, this, 10));
    [(_dec175 = [
      property
    ], "bursts")] = (__runInitializers(_init37, 71, this), __runInitializers(_init37, 72, this, []));
    [(_dec176 = [
      property
    ], "shape")] = (__runInitializers(_init37, 75, this), __runInitializers(_init37, 76, this, { type: "cone", angle: 25, radius: 1 }));
    [(_dec177 = [
      property
    ], "sizeOverLifetime")] = (__runInitializers(_init37, 79, this), __runInitializers(_init37, 80, this, null));
    [(_dec178 = [
      property
    ], "colorOverLifetime")] = (__runInitializers(_init37, 83, this), __runInitializers(_init37, 84, this, null));
    [(_dec179 = [
      property
    ], "texture")] = (__runInitializers(_init37, 87, this), __runInitializers(_init37, 88, this, null));
    [(_dec180 = [
      property
    ], "blendMode")] = (__runInitializers(_init37, 91, this), __runInitializers(_init37, 92, this, "alpha"));
    [(_dec181 = [
      property
    ], "textureSheet")] = (__runInitializers(_init37, 95, this), __runInitializers(_init37, 96, this, null));
    [(_dec182 = [
      property
    ], "stopAction")] = (__runInitializers(_init37, 99, this), __runInitializers(_init37, 100, this, "none"));
    onStopped = (__runInitializers(_init37, 103, this), undefined);
    _particleCount = 0;
    alive = false;
    paused = false;
    sent = "";
    get particleCount() {
      return this._particleCount;
    }
    get isPlaying() {
      return this.alive && !this.paused;
    }
    get isPaused() {
      return this.paused;
    }
    pairs() {
      const range = (m) => typeof m === "number" ? m : [m[0], m[1]];
      const c = this.startColor;
      const colors = c === null ? [1, 1, 1, 1] : ("r" in c) ? [c.r, c.g, c.b, c.a] : c.flatMap((x) => [x.r, x.g, x.b, x.a]);
      const s = this.shape;
      const shape = s.type === "point" ? ["point"] : s.type === "cone" ? ["cone", s.angle, s.radius] : s.type === "box" ? ["box", s.size.x, s.size.y, s.size.z] : [s.type, s.radius];
      const out = [
        "duration",
        this.duration,
        "looping",
        this.loop,
        "startDelay",
        this.startDelay,
        "lifetime",
        range(this.startLifetime),
        "speed",
        range(this.startSpeed),
        "size",
        range(this.startSize),
        "rotation",
        range(this.startRotation),
        "angularVelocity",
        range(this.angularVelocity),
        "color",
        colors,
        "gravity",
        this.gravityModifier,
        "drag",
        this.drag,
        "worldSpace",
        this.simulationSpace === "world",
        "maxParticles",
        this.maxParticles,
        "rate",
        this.rateOverTime,
        "bursts",
        this.bursts.flatMap((b) => [b.time, b.count, b.cycles ?? 1, b.interval ?? 0.01]),
        "shape",
        shape,
        "texture",
        this.texture?.id ?? null,
        "additive",
        this.blendMode === "additive",
        "seed",
        this.randomSeed
      ];
      if (this.sizeOverLifetime)
        out.push("sizeOverLifetime", this.sizeOverLifetime.flatMap((k) => [k.time, k.value]));
      if (this.colorOverLifetime) {
        out.push("colorOverLifetime", this.colorOverLifetime.flatMap((k) => [k.time, k.color.r, k.color.g, k.color.b, k.color.a]));
      }
      const sheet = this.textureSheet;
      if (sheet)
        out.push("sheet", [sheet.tilesX, sheet.tilesY, sheet.cycles ?? 1]);
      return out;
    }
    sync(playOnAwake = false) {
      const pairs = this.pairs();
      const key = JSON.stringify(pairs);
      if (key === this.sent)
        return;
      this.sent = key;
      __iron_op("render.particles", this.id, this.gameObject.id, pairs, playOnAwake);
    }
    control(action, count) {
      this.sync();
      __iron_op("render.particleControl", this.id, action, ...count === undefined ? [] : [count]);
    }
    play() {
      this.paused = false;
      this.alive = true;
      this.control("play");
    }
    stop(clear = false) {
      this.control(clear ? "stopClear" : "stop");
    }
    pause() {
      this.paused = true;
      this.control("pause");
    }
    clear() {
      this.control("clear");
    }
    emit(count) {
      this.control("emit", count);
    }
    onAwake() {
      systems.set(this.id, this);
      this.alive = this.playOnAwake;
      this.sync(this.playOnAwake);
    }
    onPreRender() {
      this.sync();
    }
    onDestroy() {
      systems.delete(this.id);
      __iron_op("render.remove", this.id);
    }
    _report(count, alive) {
      const wasAlive = this.alive;
      this._particleCount = count;
      this.alive = alive;
      if (wasAlive && !alive) {
        try {
          this.onStopped?.();
        } catch (e) {
          console.error(`[particles] error in onStopped: ${e instanceof Error ? e.message : String(e)}`);
        }
        if (this.stopAction === "destroy")
          this.gameObject.destroy();
      }
    }
  }
  globalThis.__iron_particles = (changed) => {
    for (let i = 0;i + 2 < changed.length; i += 3) {
      systems.get(changed[i])?._report(changed[i + 1], changed[i + 2] === 1);
    }
  };
  // ../../js/engine/src/textures.ts
  var nextTexture = 1;
  var textures = new Map;

  class Texture {
    id;
    src;
    width = 0;
    height = 0;
    isLoaded = false;
    failed = false;
    _filterMode;
    _wrapMode;
    _mipmaps;
    _anisoLevel;
    linear;
    pixels = null;
    waiting = [];
    constructor(src, options) {
      this.id = nextTexture++;
      this.src = src;
      this._filterMode = options.filterMode ?? "bilinear";
      this._wrapMode = options.wrapMode ?? "repeat";
      this._mipmaps = options.mipmaps ?? true;
      this._anisoLevel = options.anisoLevel ?? 1;
      this.linear = options.linear ?? false;
      textures.set(this.id, this);
    }
    static _engine(id, width, height) {
      const t = new Texture(null, {});
      textures.delete(t.id);
      nextTexture--;
      t.id = id;
      t.width = width;
      t.height = height;
      t.isLoaded = true;
      return t;
    }
    static load(src, options = {}) {
      const path = Assets.resolve(src);
      const imported = Assets.importer(path);
      const pick = (k) => options[k] ?? imported[k];
      const t = new Texture(path, {
        filterMode: pick("filterMode"),
        wrapMode: pick("wrapMode"),
        mipmaps: pick("mipmaps"),
        anisoLevel: pick("anisoLevel"),
        linear: pick("linear")
      });
      t.send();
      return t;
    }
    static create(width, height, colors, options = {}) {
      if (colors.length !== width * height) {
        throw new Error(`Texture.create: ${colors.length} colours for ${width}x${height}`);
      }
      const t = new Texture(null, options);
      const px = new Array(width * height * 4);
      const byte = options.linear ? (v) => Math.round(Mathf.clamp01(v) * 255) : (v) => Math.round(Mathf.clamp01(Mathf.linearToGammaSpace(Mathf.clamp01(v))) * 255);
      for (let y = 0;y < height; y++) {
        const row = height - 1 - y;
        for (let x = 0;x < width; x++) {
          const c = colors[y * width + x];
          const i = (row * width + x) * 4;
          px[i] = byte(c.r);
          px[i + 1] = byte(c.g);
          px[i + 2] = byte(c.b);
          px[i + 3] = Math.round(Mathf.clamp01(c.a) * 255);
        }
      }
      t.pixels = px;
      t.width = width;
      t.height = height;
      t.send();
      return t;
    }
    get filterMode() {
      return this._filterMode;
    }
    set filterMode(f) {
      this._filterMode = f;
      this.send();
    }
    get wrapMode() {
      return this._wrapMode;
    }
    set wrapMode(w) {
      this._wrapMode = w;
      this.send();
    }
    get anisoLevel() {
      return this._anisoLevel;
    }
    set anisoLevel(n) {
      this._anisoLevel = Math.min(16, Math.max(1, Math.round(n)));
      this.send();
    }
    whenLoaded() {
      if (this.isLoaded)
        return Promise.resolve(this);
      if (this.failed)
        return Promise.reject(new Error(`texture ${this.src ?? this.id} failed to load`));
      return new Promise((resolve, reject) => {
        this.waiting.push((t) => t.failed ? reject(new Error(`texture ${t.src ?? t.id} failed to load`)) : resolve(t));
      });
    }
    destroy() {
      textures.delete(this.id);
      __iron_op("render.textureRemove", this.id);
    }
    send() {
      __iron_op("render.texture", this.id, this.src, this._filterMode, this._wrapMode, this._mipmaps, ...this.pixels ? [this.width, this.height, this.pixels] : [null, null, null], this._anisoLevel, this.linear);
    }
    _loaded(width, height) {
      this.failed = width === 0;
      this.isLoaded = !this.failed;
      if (!this.failed) {
        this.width = width;
        this.height = height;
      }
      const waiting = this.waiting;
      this.waiting = [];
      for (const f of waiting)
        f(this);
    }
  }

  class RenderTexture extends Texture {
    constructor(width, height) {
      super(null, { wrapMode: "clamp", mipmaps: false });
      this.width = Math.max(1, Math.round(width));
      this.height = Math.max(1, Math.round(height));
      this.isLoaded = true;
      this.send();
    }
    setSize(width, height) {
      this.width = Math.max(1, Math.round(width));
      this.height = Math.max(1, Math.round(height));
      this.send();
    }
    send() {
      if (!this.isLoaded)
        return;
      __iron_op("render.renderTexture", this.id, this.width, this.height);
    }
  }
  globalThis.__iron_textures = (loaded) => {
    for (const [id, w, h] of loaded)
      textures.get(id)?._loaded(w, h);
  };

  class Sprite {
    texture;
    rect;
    pivot;
    pixelsPerUnit;
    constructor(texture, rect, pivot, pixelsPerUnit) {
      this.texture = texture;
      this.rect = rect;
      this.pivot = pivot;
      this.pixelsPerUnit = pixelsPerUnit;
    }
    static create(texture, rect = null, pivot = new Vector2(0.5, 0.5), pixelsPerUnit = 100) {
      return new Sprite(texture, rect, new Vector2(pivot.x, pivot.y), pixelsPerUnit);
    }
    get size() {
      const w = this.rect?.width ?? this.texture.width;
      const h = this.rect?.height ?? this.texture.height;
      return new Vector2(w / this.pixelsPerUnit, h / this.pixelsPerUnit);
    }
    static grid(texture, columns, rows, pivot, pixelsPerUnit) {
      const w = texture.width / columns;
      const h = texture.height / rows;
      const out = [];
      for (let r = 0;r < rows; r++) {
        for (let c = 0;c < columns; c++) {
          out.push(Sprite.create(texture, new Rect(c * w, texture.height - (r + 1) * h, w, h), pivot, pixelsPerUnit));
        }
      }
      return out;
    }
  }
  var _init38, _base38, _dec183, _dec184, _dec185, _dec186, _dec187;

  class SpriteRenderer extends (_base38 = Component) {
    static {
      _init38 = __decoratorStart(_base38);
      __decorateElement(_init38, 2, "sprite", _dec183, this);
      __decorateElement(_init38, 2, "color", _dec184, this);
      __decorateElement(_init38, 2, "flipX", _dec185, this);
      __decorateElement(_init38, 2, "flipY", _dec186, this);
      __decorateElement(_init38, 2, "sortingOrder", _dec187, this);
      __decoratorMetadata(_init38, this);
    }
    _sprite = (__runInitializers(_init38, 5, this), null);
    _color = Color.white;
    _flipX = false;
    _flipY = false;
    _sortingOrder = 0;
    started = false;
    shown = false;
    get [(_dec183 = [
      property
    ], "sprite")]() {
      return this._sprite;
    }
    set sprite(s) {
      this._sprite = s;
      this.send();
    }
    get [(_dec184 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.send();
    }
    get [(_dec185 = [
      property
    ], "flipX")]() {
      return this._flipX;
    }
    set flipX(f) {
      this._flipX = f;
      this.send();
    }
    get [(_dec186 = [
      property
    ], "flipY")]() {
      return this._flipY;
    }
    set flipY(f) {
      this._flipY = f;
      this.send();
    }
    get [(_dec187 = [
      property
    ], "sortingOrder")]() {
      return this._sortingOrder;
    }
    set sortingOrder(o) {
      this._sortingOrder = Math.trunc(o);
      this.send();
    }
    send() {
      if (!this.started)
        return;
      const s = this._sprite;
      if (!s) {
        if (this.shown)
          __iron_op("render.remove", this.id);
        this.shown = false;
        return;
      }
      const c = this._color;
      const r = s.rect;
      __iron_op("render.sprite", this.id, this.gameObject.id, s.texture.id, r ? [r.x, r.y, r.width, r.height] : null, [s.pivot.x, s.pivot.y], s.pixelsPerUnit, [c.r, c.g, c.b, c.a], this._flipX, this._flipY, this._sortingOrder);
      this.shown = true;
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
  }
  // ../../js/engine/src/input.ts
  var keys = new Set;
  var pressedPending = new Set;
  var releasedPending = new Set;
  var pressedThisTick = new Set;
  var releasedThisTick = new Set;
  var mouseX = 0;
  var mouseY = 0;
  var deltaPending = { x: 0, y: 0 };
  var wheelPending = { x: 0, y: 0 };
  var deltaThisTick = { x: 0, y: 0 };
  var wheelThisTick = { x: 0, y: 0 };
  onTick(() => {
    pressedThisTick = pressedPending;
    releasedThisTick = releasedPending;
    pressedPending = new Set;
    releasedPending = new Set;
    deltaThisTick = deltaPending;
    wheelThisTick = wheelPending;
    deltaPending = { x: 0, y: 0 };
    wheelPending = { x: 0, y: 0 };
  });
  var pads = new Map;
  var listeners3 = {};
  function emit(type, event) {
    for (const fn of listeners3[type] ?? []) {
      try {
        fn(event);
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[input] error in ${type} listener: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  }
  function pad(id, name = "") {
    let p = pads.get(id);
    if (!p) {
      p = { id, name, connected: true, buttons: {}, axes: {} };
      pads.set(id, p);
    }
    return p;
  }
  globalThis.__iron_input = (type, ...args) => {
    switch (type) {
      case "keydown": {
        const [code, key, repeat] = args;
        if (!repeat && !keys.has(code))
          pressedPending.add(code);
        keys.add(code);
        emit("keydown", { code, key, repeat: !!repeat });
        break;
      }
      case "keyup": {
        const [code, key] = args;
        if (keys.has(code))
          releasedPending.add(code);
        keys.delete(code);
        emit("keyup", { code, key, repeat: false });
        break;
      }
      case "mousemove":
        [mouseX, mouseY] = args;
        break;
      case "mousedelta": {
        const [dx, dy] = args;
        deltaPending.x += dx;
        deltaPending.y += dy;
        break;
      }
      case "wheel": {
        const [dx, dy] = args;
        wheelPending.x += dx;
        wheelPending.y += dy;
        break;
      }
      case "blur":
        keys.clear();
        for (const p of pads.values())
          for (const b of Object.keys(p.buttons))
            p.buttons[b] = 0;
        break;
      case "gamepadconnected": {
        const [id, name] = args;
        const p = pad(id, name);
        p.connected = true;
        emit("gamepadconnected", { gamepad: p });
        break;
      }
      case "gamepaddisconnected": {
        const p = pad(args[0]);
        p.connected = false;
        emit("gamepaddisconnected", { gamepad: p });
        break;
      }
      case "gamepadbutton": {
        const [id, button, value] = args;
        const p = pad(id);
        const was = (p.buttons[button] ?? 0) > 0.5;
        p.buttons[button] = value;
        const now = value > 0.5;
        if (now && !was)
          emit("gamepadbuttondown", { gamepad: p, button });
        if (!now && was)
          emit("gamepadbuttonup", { gamepad: p, button });
        break;
      }
      case "gamepadaxis": {
        const [id, axis, value] = args;
        const p = pad(id);
        p.axes[axis] = value;
        emit("gamepadaxis", { gamepad: p, axis, value });
        break;
      }
    }
  };
  // ../../js/engine/src/prefs.ts
  var values = new Map(Object.entries(globalThis.__iron_info?.prefs ?? {}));
  function set(key, value) {
    values.set(key, value);
    __iron_op("prefs.set", key, value);
  }
  function _loadJson(key) {
    const v = values.get(key);
    if (typeof v !== "string")
      return null;
    try {
      return JSON.parse(v);
    } catch {
      return null;
    }
  }
  function _saveJson(key, value) {
    set(key, JSON.stringify(value));
  }

  // ../../js/engine/src/quality.ts
  var DEFAULT_LEVELS = [
    { name: "Low", antiAliasing: 1, renderScale: 0.75, shadows: "hardOnly", softShadowQuality: "low", shadowResolution: 1024, shadowDistance: 20, shadowCascades: 1, lodBias: 0.4, vSyncCount: 1, anisotropicFiltering: "disable", globalTextureMipmapLimit: 1 },
    { name: "Medium", antiAliasing: 2, renderScale: 1, shadows: "all", softShadowQuality: "low", shadowResolution: 2048, shadowDistance: 25, shadowCascades: 2, lodBias: 0.7, vSyncCount: 1, anisotropicFiltering: "enable", globalTextureMipmapLimit: 0 },
    { name: "High", antiAliasing: 4, renderScale: 1, shadows: "all", softShadowQuality: "low", shadowResolution: 2048, shadowDistance: 30, shadowCascades: 4, lodBias: 1, vSyncCount: 1, anisotropicFiltering: "enable", globalTextureMipmapLimit: 0 },
    { name: "Ultra", antiAliasing: 8, renderScale: 1, shadows: "all", softShadowQuality: "high", shadowResolution: 4096, shadowDistance: 40, shadowCascades: 4, lodBias: 2, vSyncCount: 1, anisotropicFiltering: "forceEnable", globalTextureMipmapLimit: 0 }
  ];
  var manifest = () => {
    const q = globalThis.__iron_info?.app?.quality;
    return q && typeof q === "object" ? q : {};
  };
  function buildLevels() {
    const overrides = manifest().levels ?? {};
    return DEFAULT_LEVELS.map((l) => ({ ...l, ...overrides[l.name] ?? {}, name: l.name }));
  }
  var levels = buildLevels();
  var levelIndex = Math.max(0, levels.findIndex((l) => l.name === (manifest().default ?? "High")));
  var current = { ...levels[levelIndex] };
  function sendQuality() {
    const c = current;
    __iron_op("render.quality", 0, c.antiAliasing, c.renderScale, c.shadows, c.softShadowQuality === "high" ? 2 : 1, c.lodBias, c.vSyncCount, c.anisotropicFiltering, c.globalTextureMipmapLimit);
    remember();
  }
  function sendShadows() {
    __iron_op("render.shadowSettings", 0, current.shadowDistance, current.shadowResolution, current.shadowCascades);
    remember();
  }
  function remember() {
    if (restoring)
      return;
    _saveJson("iron.quality", { level: levels[levelIndex]?.name, current });
  }
  var restoring = false;
  var listeners4 = new Set;
  function _qualityState() {
    return { level: levelIndex, current };
  }
  function _restoreQuality(state) {
    const s = state;
    if (!s?.current)
      return;
    levelIndex = Math.min(levels.length - 1, Math.max(0, s.level ?? levelIndex));
    current = { ...levels[levelIndex], ...s.current };
    sendQuality();
    sendShadows();
  }
  function _applyDefaultQuality() {
    const saved = _loadJson("iron.quality");
    if (saved?.current) {
      const i = levels.findIndex((l) => l.name === saved.level);
      if (i >= 0)
        levelIndex = i;
      current = { ...levels[levelIndex], ...saved.current, name: levels[levelIndex].name };
    }
    restoring = true;
    try {
      sendQuality();
      sendShadows();
    } finally {
      restoring = false;
    }
  }
  if (typeof __iron_op === "function")
    _applyDefaultQuality();
  // ../../js/engine/src/models.ts
  var nextModel = 1;
  var models = new Map;
  var cache2 = new Map;

  class Model {
    id;
    src;
    isLoaded = false;
    failed = false;
    materials = [];
    meshes = [];
    animations = [];
    description = null;
    waiting = [];
    pending = [];
    constructor(src) {
      this.id = nextModel++;
      this.src = src;
      models.set(this.id, this);
      __iron_op("render.model", this.id, src);
    }
    static load(source) {
      const src = Assets.resolve(source);
      let m = cache2.get(src);
      if (!m) {
        m = new Model(src);
        cache2.set(src, m);
      }
      return m;
    }
    whenLoaded() {
      if (this.isLoaded)
        return Promise.resolve(this);
      if (this.failed)
        return Promise.reject(new Error(`model ${this.src} failed to load`));
      return new Promise((resolve, reject) => {
        this.waiting.push((m) => m.failed ? reject(new Error(`model ${m.src} failed to load`)) : resolve(m));
      });
    }
    instantiate(options = {}) {
      const name = this.src.split("/").pop()?.replace(/\.(glb|gltf)$/i, "") ?? "Model";
      const root = new GameObject(name, {
        position: options.position,
        rotation: options.rotation,
        scale: options.scale,
        parent: options.parent
      });
      this._buildInto(root, options);
      return root;
    }
    _buildInto(root, options = {}) {
      root._model = options.colliders && options.colliders !== "none" ? { src: this.src, colliders: options.colliders } : { src: this.src };
      if (this.description)
        this.build(root, options);
      else if (!this.failed)
        this.pending.push({ options, root });
    }
    build(root, options) {
      const d = this.description;
      const objects = new Map;
      const skinned = [];
      const make = (index, parent) => {
        const n = d.nodes[index];
        const go = new GameObject(n.name || `Node ${index}`, {
          parent,
          position: new Vector3(...n.position),
          rotation: new Quaternion(...n.rotation),
          scale: new Vector3(...n.scale)
        });
        go._modelPart = true;
        objects.set(index, go);
        n.primitives.forEach((p, i) => {
          const holder = n.primitives.length === 1 ? go : new GameObject(`${go.name} ${i}`, { parent: go });
          holder._modelPart = true;
          const mesh = Mesh._engine(p.mesh);
          const material = p.material === null ? new Material : this.materials[p.material] ?? new Material;
          const skin = n.skin == null ? undefined : d.skins?.[n.skin];
          if (skin)
            skinned.push({ holder, mesh, material, skin });
          else
            holder.addComponent(MeshRenderer, { mesh, material });
          if (options.colliders === "mesh" || options.colliders === "convex") {
            holder.addComponent(MeshCollider, { sharedMesh: mesh, convex: options.colliders === "convex" });
          }
        });
        for (const c of n.children)
          make(c, go);
      };
      for (const r of d.roots)
        make(r, root);
      for (const s of skinned) {
        s.mesh.bindposes = Array.from({ length: s.skin.joints.length }, (_, i) => new Matrix4x4(s.skin.inverseBind.slice(i * 16, i * 16 + 16)));
        const bones = s.skin.joints.map((j) => objects.get(j)).filter((b) => b !== undefined);
        s.holder.addComponent(SkinnedMeshRenderer, { mesh: s.mesh, material: s.material, bones, rootBone: bones[0] ?? null });
      }
      if (this.animations.length && !root.getComponent(Animation)) {
        root.addComponent(Animation, { clip: this.animations[0], clips: [...this.animations] });
      }
    }
    clips(d) {
      const paths = new Map;
      const walk = (index, prefix) => {
        const n = d.nodes[index];
        const path = prefix + (n.name || `Node ${index}`);
        paths.set(index, path);
        for (const c of n.children)
          walk(c, `${path}/`);
      };
      for (const r of d.roots)
        walk(r, "");
      return (d.animations ?? []).map((a, i) => {
        const clip = new AnimationClip(a.name || `Animation ${i}`);
        clip.wrapMode = "loop";
        for (const ch of a.channels) {
          const path = paths.get(ch.node);
          if (path === undefined)
            continue;
          const [property, size] = ch.path === "translation" ? ["localPosition", 3] : ch.path === "rotation" ? ["localRotation", 4] : ["localScale", 3];
          for (let k = 0;k < size; k++) {
            const curve = ch.interpolation === "cubic" ? cubic(ch.times, ch.values, size, k) : AnimationCurve.fromPoints(ch.times.map((t, i) => [t, ch.values[i * size + k]]), ch.interpolation === "step" ? "step" : "linear");
            clip.setCurve(path, "Transform", `${property}.${"xyzw"[k]}`, curve);
          }
        }
        return clip;
      });
    }
    _loaded(description) {
      if (description) {
        this.description = description;
        this.isLoaded = true;
        this.materials = description.materials.map((m) => {
          const [r, g, b, a] = m.color;
          const texture = m.texture ? Texture._engine(...m.texture) : null;
          const map = (id) => id == null ? null : Texture._engine(id, 0, 0);
          return new Material(new Color(r, g, b, a), new Color(m.emission[0], m.emission[1], m.emission[2], 1), m.smoothness, m.unlit, texture, Vector2.one, Vector2.zero, m.metallic ?? 0).with({
            renderingMode: m.alpha === "mask" ? "cutout" : m.alpha === "blend" ? "fade" : "opaque",
            alphaCutoff: m.alphaCutoff ?? 0.5,
            doubleSided: m.doubleSided ?? false,
            normalMap: map(m.normal?.[0]),
            normalScale: m.normal?.[1] ?? 1,
            metallicGlossMap: map(m.metallicRoughness),
            occlusionMap: map(m.occlusion?.[0]),
            occlusionStrength: m.occlusion?.[1] ?? 1,
            emissionMap: map(m.emissive),
            gltfPacking: true
          });
        });
        this.meshes = description.nodes.flatMap((n) => n.primitives.map((p) => Mesh._engine(p.mesh)));
        this.animations = this.clips(description);
        for (const { root, options } of this.pending)
          if (!root.destroyed)
            this.build(root, options);
      } else {
        this.failed = true;
      }
      this.pending = [];
      const waiting = this.waiting;
      this.waiting = [];
      for (const f of waiting)
        f(this);
    }
  }
  globalThis.__iron_models = (loaded) => {
    for (let i = 0;i + 1 < loaded.length; i += 2) {
      const m = models.get(loaded[i]);
      const json = loaded[i + 1];
      m?._loaded(typeof json === "string" ? JSON.parse(json) : null);
    }
  };
  function cubic(times, values, size, k) {
    const keys = times.map((t, i) => {
      const at = (part) => values[(i * 3 + part) * size + k];
      return { t, inT: at(0), v: at(1), outT: at(2) };
    });
    const curve = new AnimationCurve;
    for (const key of keys)
      curve.addKey(key.t, key.v);
    curve.keys.forEach((key, i) => {
      key.inTangent = keys[i].inT;
      key.outTangent = keys[i].outT;
    });
    return curve;
  }
  // ../../js/engine/src/scenes.ts
  _registerBuiltIns({
    AudioListener,
    AudioSource,
    BoxCollider,
    BoxCollider2D,
    Camera,
    CameraFollow,
    CameraOrbit,
    CapsuleCollider,
    CapsuleCollider2D,
    Animation,
    Animator,
    CharacterController,
    CharacterJoint,
    Cloth,
    CircleCollider2D,
    EdgeCollider2D,
    FixedJoint,
    HingeJoint,
    Light,
    LineRenderer,
    LODGroup,
    MeshCollider,
    MeshRenderer,
    SkinnedMeshRenderer,
    Mirror,
    ParticleSystem,
    PlaneRenderer,
    PolygonCollider2D,
    ReflectionProbe,
    Rigidbody,
    Rigidbody2D,
    Rope,
    RopeJoint,
    SphereCollider,
    SpringJoint,
    SpriteRenderer,
    WorldPanel
  });
  function fromGuid(guid, src) {
    return typeof guid === "string" && Assets.guidToPath(guid) || String(src);
  }
  var warn = (message) => console.warn(`scene: ${message}`);
  var UNSAVED = Symbol("unsaved");
  var MATERIAL_EXTRAS = [
    "normalMap",
    "normalScale",
    "metallicGlossMap",
    "specularColor",
    "occlusionMap",
    "occlusionStrength",
    "emissionMap",
    "detailAlbedoMap",
    "detailAlbedoMapScale",
    "detailAlbedoMapOffset",
    "renderingMode",
    "alphaCutoff",
    "doubleSided",
    "gltfPacking"
  ];
  var DEFAULT_MATERIAL = new Material;

  class Saver {
    keepExternal;
    reload;
    ids = new Map;
    assets = {};
    assetIds = new Map;
    constructor(keepExternal, reload = false) {
      this.keepExternal = keepExternal;
      this.reload = reload;
    }
    field(v, where) {
      if (typeof v === "function" || typeof v === "symbol" || typeof v === "bigint")
        return UNSAVED;
      this.unsaved = false;
      const out = this.value(v, where);
      return this.unsaved ? UNSAVED : out;
    }
    unsaved = false;
    id(thing) {
      let id = this.ids.get(thing);
      if (id === undefined) {
        id = this.ids.size + 1;
        this.ids.set(thing, id);
      }
      return id;
    }
    asset(key, prefix, make) {
      let id = this.assetIds.get(key);
      if (!id) {
        id = `${prefix}${this.assetIds.size + 1}`;
        this.assetIds.set(key, id);
        this.assets[id] = make();
      }
      return { $asset: id };
    }
    value(v, where) {
      if (v === null || v === undefined || typeof v === "number" || typeof v === "string" || typeof v === "boolean") {
        return v ?? null;
      }
      if (Array.isArray(v))
        return v.map((x, i) => this.value(x, `${where}[${i}]`));
      if (v instanceof Vector2Int)
        return { $type: "Vector2Int", value: [v.x, v.y] };
      if (v instanceof Vector3Int)
        return { $type: "Vector3Int", value: [v.x, v.y, v.z] };
      if (v instanceof Vector2)
        return { $type: "Vector2", value: [v.x, v.y] };
      if (v instanceof Vector3)
        return { $type: "Vector3", value: [v.x, v.y, v.z] };
      if (v instanceof Vector4)
        return { $type: "Vector4", value: [v.x, v.y, v.z, v.w] };
      if (v instanceof Quaternion)
        return { $type: "Quaternion", value: [v.x, v.y, v.z, v.w] };
      if (v instanceof Color)
        return { $type: "Color", value: [v.r, v.g, v.b, v.a] };
      if (v instanceof RectInt)
        return { $type: "RectInt", value: [v.x, v.y, v.width, v.height] };
      if (v instanceof Rect)
        return { $type: "Rect", value: [v.x, v.y, v.width, v.height] };
      if (v instanceof GameObject || v instanceof Component) {
        const id = this.ids.get(v);
        const kind = v instanceof GameObject ? "GameObject" : "Component";
        if (id !== undefined)
          return { $ref: kind, id };
        if (this.keepExternal)
          return { $ref: kind, live: v instanceof GameObject ? v.id : v.id };
        return null;
      }
      if (v instanceof AnimationCurve)
        return { $type: "AnimationCurve", data: v._toJSON() };
      if (v instanceof AnimationClip)
        return { $type: "AnimationClip", data: v.toJSON() };
      if (v instanceof AnimatorController)
        return { $type: "AnimatorController", data: v.toJSON() };
      if (v instanceof Material) {
        return {
          $type: "Material",
          color: this.value(v.color, where),
          emission: this.value(v.emission, where),
          smoothness: v.smoothness,
          metallic: v.metallic,
          unlit: v.unlit,
          mainTexture: this.value(v.mainTexture, where),
          mainTextureScale: this.value(v.mainTextureScale, where),
          mainTextureOffset: this.value(v.mainTextureOffset, where),
          ...Object.fromEntries(MATERIAL_EXTRAS.filter((k) => {
            const d = DEFAULT_MATERIAL[k];
            const x = v[k];
            return !(x === d || x instanceof Vector2 && d instanceof Vector2 && x.equals(d));
          }).map((k) => [k, this.value(v[k], where)]))
        };
      }
      if (v instanceof RenderTexture) {
        return this.asset(v, "texture", () => ({ type: "RenderTexture", width: v.width, height: v.height }));
      }
      if (v instanceof Texture) {
        if (!v.src)
          return this.cannot(`${where}: only textures loaded from files are saved`);
        const t = v;
        const guid = Assets.pathToGuid(v.src);
        return this.asset(v, "texture", () => ({
          type: "Texture",
          src: v.src,
          ...guid ? { guid } : {},
          filterMode: v.filterMode,
          wrapMode: v.wrapMode,
          mipmaps: t._mipmaps,
          ...v.anisoLevel !== 1 ? { anisoLevel: v.anisoLevel } : {},
          ...v.linear ? { linear: true } : {}
        }));
      }
      if (v instanceof Sprite) {
        return { $type: "Sprite", texture: this.value(v.texture, where), rect: this.value(v.rect, where), pivot: this.value(v.pivot, where), pixelsPerUnit: v.pixelsPerUnit };
      }
      if (v instanceof Mesh) {
        const data = v._data;
        if (!data)
          return this.cannot(`${where}: only meshes made with Mesh.create are saved (model meshes come with their model)`);
        return this.asset(v, "mesh", () => ({
          type: "Mesh",
          ...data,
          ...v.bindposes.length ? { bindposes: v.bindposes.flatMap((m) => [...m.m]) } : {}
        }));
      }
      if (v instanceof Sound) {
        const guid = Assets.pathToGuid(v.src);
        return { $type: "Sound", src: v.src, ...guid ? { guid } : {} };
      }
      const proto = Object.getPrototypeOf(v);
      if (proto === Object.prototype || proto === null) {
        const out = {};
        for (const [k, x] of Object.entries(v)) {
          if (x !== undefined)
            out[k] = this.value(x, `${where}.${k}`);
        }
        return out;
      }
      return this.cannot(`${where}: can't save a ${proto?.constructor?.name ?? "value"}`);
    }
    cannot(message) {
      if (this.reload)
        this.unsaved = true;
      else
        warn(message);
      return null;
    }
  }

  class Loader {
    data;
    things = new Map;
    assets = new Map;
    constructor(data = {}) {
      this.data = data;
    }
    asset(id) {
      if (this.assets.has(id))
        return this.assets.get(id);
      const a = this.data[id];
      let made = null;
      if (a?.type === "RenderTexture") {
        made = new RenderTexture(Number(a.width), Number(a.height));
      } else if (a?.type === "Texture") {
        made = Texture.load(fromGuid(a.guid, a.src), {
          filterMode: a.filterMode,
          wrapMode: a.wrapMode,
          mipmaps: a.mipmaps,
          anisoLevel: a.anisoLevel,
          linear: a.linear
        });
      } else if (a?.type === "Mesh") {
        const flat = (k) => a[k] ?? [];
        const v3 = (f) => Array.from({ length: f.length / 3 }, (_, i) => new Vector3(f[i * 3], f[i * 3 + 1], f[i * 3 + 2]));
        const uv = flat("uv");
        const joints = flat("joints");
        const weights = flat("weights");
        const four = (f, i) => [f[i * 4], f[i * 4 + 1], f[i * 4 + 2], f[i * 4 + 3]];
        const poses = flat("bindposes");
        made = Mesh.create({
          vertices: v3(flat("vertices")),
          triangles: flat("triangles"),
          normals: v3(flat("normals")),
          uv: Array.from({ length: uv.length / 2 }, (_, i) => new Vector2(uv[i * 2], uv[i * 2 + 1])),
          boneWeights: Array.from({ length: joints.length / 4 }, (_, i) => ({ indices: four(joints, i), weights: four(weights, i) })),
          bindposes: Array.from({ length: poses.length / 16 }, (_, i) => new Matrix4x4(poses.slice(i * 16, i * 16 + 16)))
        });
      } else {
        warn(`unknown asset ${id}`);
      }
      this.assets.set(id, made);
      return made;
    }
    value(v) {
      if (v === null || typeof v !== "object")
        return v;
      if (Array.isArray(v))
        return v.map((x) => this.value(x));
      const o = v;
      if (typeof o.$asset === "string")
        return this.asset(o.$asset);
      if (typeof o.$ref === "string") {
        if (typeof o.live === "number") {
          return o.$ref === "GameObject" ? GameObject.byId(o.live) ?? null : liveComponent(o.live) ?? null;
        }
        const found = this.things.get(o.id);
        return found ?? null;
      }
      if (typeof o.$type === "string") {
        const n = o.value ?? [];
        switch (o.$type) {
          case "Vector2":
            return new Vector2(n[0], n[1]);
          case "Vector2Int":
            return new Vector2Int(n[0], n[1]);
          case "Vector3":
            return new Vector3(n[0], n[1], n[2]);
          case "Vector3Int":
            return new Vector3Int(n[0], n[1], n[2]);
          case "Vector4":
            return new Vector4(n[0], n[1], n[2], n[3]);
          case "Quaternion":
            return new Quaternion(n[0], n[1], n[2], n[3]);
          case "Color":
            return new Color(n[0], n[1], n[2], n[3]);
          case "Rect":
            return new Rect(n[0], n[1], n[2], n[3]);
          case "RectInt":
            return new RectInt(n[0], n[1], n[2], n[3]);
          case "AnimationCurve":
            return AnimationCurve._fromJSON(o.data);
          case "AnimationClip":
            return AnimationClip.fromJSON(o.data);
          case "AnimatorController":
            return AnimatorController.fromJSON(o.data);
          case "Material":
            return Material.create({
              color: this.value(o.color),
              emission: this.value(o.emission),
              smoothness: o.smoothness,
              metallic: o.metallic,
              unlit: o.unlit,
              mainTexture: this.value(o.mainTexture),
              mainTextureScale: this.value(o.mainTextureScale),
              mainTextureOffset: this.value(o.mainTextureOffset),
              ...Object.fromEntries(MATERIAL_EXTRAS.filter((k) => (k in o)).map((k) => [k, this.value(o[k])]))
            });
          case "Sprite":
            return Sprite.create(this.value(o.texture), this.value(o.rect), this.value(o.pivot), o.pixelsPerUnit);
          case "Sound":
            return new Sound(fromGuid(o.guid, o.src));
          default:
            warn(`unknown type ${o.$type}`);
            return null;
        }
      }
      const out = {};
      for (const [k, x] of Object.entries(o))
        out[k] = this.value(x);
      return out;
    }
  }
  function liveComponent(id) {
    for (const root of GameObject.roots) {
      const stack = [root];
      while (stack.length) {
        const o = stack.pop();
        const c = o.components.find((c) => c.id === id);
        if (c)
          return c;
        stack.push(...o.children);
      }
    }
    return;
  }
  function number(saver, roots) {
    const walk = (go) => {
      if (go._modelPart || unsaved(go))
        return;
      saver.id(go);
      for (const c of go.components)
        saver.id(c);
      for (const child of go.children)
        walk(child);
    };
    for (const r of roots)
      walk(r);
  }
  var unsaved = (go) => (go.hideFlags & HideFlags.DontSave) !== 0;
  function saveObject(saver, go) {
    if (go._modelPart || unsaved(go))
      return null;
    const t = go.transform;
    const p = t.localPosition;
    const r = t.localRotation;
    const s = t.localScale;
    const data = { id: saver.id(go), name: go.name };
    if (p.x !== 0 || p.y !== 0 || p.z !== 0)
      data.position = [p.x, p.y, p.z];
    if (r.x !== 0 || r.y !== 0 || r.z !== 0 || r.w !== 1)
      data.rotation = [r.x, r.y, r.z, r.w];
    if (s.x !== 1 || s.y !== 1 || s.z !== 1)
      data.scale = [s.x, s.y, s.z];
    if (!go.enabled)
      data.enabled = false;
    if (go.layer !== 0)
      data.layer = go.layer;
    if (go._model) {
      const guid = Assets.pathToGuid(go._model.src);
      data.model = { ...go._model, ...guid ? { guid } : {} };
    }
    if (go._prefab)
      data.prefab = go._prefab;
    const components = go.components.map((c) => {
      const cls = c.constructor;
      const properties = {};
      const own = c;
      const props = propertiesOf(cls);
      for (const name of props) {
        const value = own[name];
        if (value !== undefined)
          properties[name] = saver.value(value, `${go.name}.${_nameOf(cls)}.${name}`);
      }
      if (saver.reload && !_isBuiltIn(cls)) {
        for (const name of Object.keys(own)) {
          if (name === "id" || name === "gameObject" || props.includes(name) || own[name] === undefined)
            continue;
          const value = saver.field(own[name], `${go.name}.${_nameOf(cls)}.${name}`);
          if (value !== UNSAVED)
            properties[name] = value;
        }
      }
      return { id: saver.id(c), type: _nameOf(cls), properties };
    });
    if (components.length)
      data.components = components;
    const children = go.children.map((c) => saveObject(saver, c)).filter((c) => c !== null);
    if (children.length)
      data.children = children;
    return data;
  }
  function loadObjects(list, assets, parent, reload = false) {
    const loader = new Loader(assets);
    const pending = [];
    const disabled = [];
    const build = (d, under) => {
      const [p, r, s] = [d.position, d.rotation, d.scale];
      const go = new GameObject(d.name ?? "GameObject", {
        parent: under,
        position: p ? new Vector3(p[0], p[1], p[2]) : undefined,
        rotation: r ? new Quaternion(r[0], r[1], r[2], r[3]) : undefined,
        scale: s ? new Vector3(s[0], s[1], s[2]) : undefined
      });
      loader.things.set(d.id, go);
      if (d.layer)
        go.layer = d.layer;
      if (d.enabled === false)
        disabled.push(go);
      if (d.model)
        Model.load(fromGuid(d.model.guid, d.model.src))._buildInto(go, { colliders: d.model.colliders });
      if (d.prefab)
        go._prefab = d.prefab;
      for (const c of d.components ?? []) {
        const cls = _classByName(c.type);
        if (!cls) {
          warn(`${d.name}: unknown component ${c.type} (register it with register({ ${c.type} }))`);
          continue;
        }
        const component = go._addComponentAsleep(cls);
        loader.things.set(c.id, component);
        pending.push({ component, properties: c.properties ?? {} });
      }
      for (const child of d.children ?? [])
        build(child, go);
      return go;
    };
    const made = list.map((d) => build(d, parent));
    for (const { component, properties } of pending) {
      const own = component;
      const cls = component.constructor;
      const props = propertiesOf(cls);
      for (const [name, value] of Object.entries(properties)) {
        if (!props.includes(name) && !Object.hasOwn(own, name)) {
          warn(`${component.gameObject.name}: ${_nameOf(cls)} has no ${name} any more; dropped`);
          continue;
        }
        try {
          own[name] = loader.value(value);
        } catch (e) {
          warn(`${component.gameObject.name}: couldn't set ${name}: ${e}`);
        }
      }
    }
    const restored = new Set(reload ? pending.map((p) => p.component).filter((c) => !_isBuiltIn(c.constructor)) : []);
    for (const component of restored)
      _markRestored(component);
    for (const { component } of pending)
      if (!restored.has(component))
        component.onAwake?.();
    for (const component of restored) {
      try {
        component.onReload?.();
      } catch (e) {
        console.error(`${component.gameObject.name}: onReload: ${e}`);
      }
    }
    for (const go of disabled)
      go.enabled = false;
    return made;
  }
  var nextHandle = 1;

  class Scene {
    name;
    handle;
    path = null;
    isLoaded = true;
    _renderSettings = null;
    constructor(name) {
      this.name = name;
      this.handle = nextHandle++;
    }
    getRootGameObjects() {
      return GameObject.roots.filter((go) => go._scene === this);
    }
    get rootCount() {
      return this.getRootGameObjects().length;
    }
    serialize() {
      return this._serialize(false);
    }
    _serialize(reload) {
      const saver = new Saver(false, reload);
      const roots = this.getRootGameObjects();
      number(saver, roots);
      const objects = roots.map((go) => saveObject(saver, go)).filter((d) => d !== null);
      const data = { format: "iron.scene", version: 1, name: this.name, objects };
      if (this === active)
        data.renderSettings = saveRenderSettings(saver);
      if (Object.keys(saver.assets).length)
        data.assets = saver.assets;
      return data;
    }
  }
  function saveRenderSettings(saver) {
    return saver.value({
      ambientLight: RenderSettings.ambientLight,
      ambientMode: RenderSettings.ambientMode,
      reflectionIntensity: RenderSettings.reflectionIntensity,
      skybox: RenderSettings.skybox,
      bloom: RenderSettings.bloom,
      fog: RenderSettings.fog,
      fogMode: RenderSettings.fogMode,
      fogColor: RenderSettings.fogColor,
      fogDensity: RenderSettings.fogDensity,
      fogStartDistance: RenderSettings.fogStartDistance,
      fogEndDistance: RenderSettings.fogEndDistance,
      heightFog: RenderSettings.heightFog,
      volumetricFog: RenderSettings.volumetricFog
    }, "renderSettings");
  }
  function applyRenderSettings(saved) {
    const s = new Loader().value(saved);
    if (s.ambientLight instanceof Color)
      RenderSettings.ambientLight = s.ambientLight;
    if (s.ambientMode === "skybox" || s.ambientMode === "color")
      RenderSettings.ambientMode = s.ambientMode;
    if (typeof s.reflectionIntensity === "number")
      RenderSettings.reflectionIntensity = s.reflectionIntensity;
    if ("skybox" in s)
      RenderSettings.skybox = s.skybox ?? null;
    if ("bloom" in s)
      RenderSettings.bloom = s.bloom ?? null;
    if (s.fogMode === "linear" || s.fogMode === "exponential" || s.fogMode === "exponentialSquared")
      RenderSettings.fogMode = s.fogMode;
    if (s.fogColor instanceof Color)
      RenderSettings.fogColor = s.fogColor;
    if (typeof s.fogDensity === "number")
      RenderSettings.fogDensity = s.fogDensity;
    if (typeof s.fogStartDistance === "number")
      RenderSettings.fogStartDistance = s.fogStartDistance;
    if (typeof s.fogEndDistance === "number")
      RenderSettings.fogEndDistance = s.fogEndDistance;
    if ("heightFog" in s)
      RenderSettings.heightFog = s.heightFog ?? null;
    if ("volumetricFog" in s)
      RenderSettings.volumetricFog = s.volumetricFog ?? null;
    if (typeof s.fog === "boolean")
      RenderSettings.fog = s.fog;
  }
  var scenes = [];
  var active;
  var keep = null;
  var loadedListeners = new Set;
  var unloadedListeners = new Set;
  var activeListeners = new Set;
  function reset() {
    scenes = [new Scene("Main")];
    active = scenes[0];
    keep = null;
    loadedListeners.clear();
    unloadedListeners.clear();
    activeListeners.clear();
  }
  reset();
  function setActive(scene) {
    if (scene === active)
      return;
    const previous = active;
    active = scene;
    if (scene._renderSettings)
      applyRenderSettings(scene._renderSettings);
    for (const f of activeListeners)
      f(previous, scene);
  }
  function unload(scene) {
    for (const go of scene.getRootGameObjects())
      go.destroy();
    scene.isLoaded = false;
    scenes = scenes.filter((s) => s !== scene);
    for (const f of unloadedListeners)
      f(scene);
  }
  function fromData(data, mode, path) {
    if (data?.format !== "iron.scene")
      throw new Error("not a scene (format isn't iron.scene)");
    if (data.version !== 1)
      throw new Error(`scene version ${data.version} isn't supported`);
    const scene = new Scene(data.name);
    scene.path = path;
    scene._renderSettings = data.renderSettings ?? null;
    const old = mode === "single" ? [...scenes] : [];
    scenes.push(scene);
    for (const s of old)
      unload(s);
    const previous = active;
    active = scene;
    try {
      loadObjects(data.objects ?? [], data.assets, null);
    } finally {
      active = previous;
    }
    if (mode === "single")
      setActive(scene);
    for (const f of loadedListeners)
      f(scene, mode);
    return scene;
  }
  var nextRead = 1;
  var reads = new Map;
  globalThis.__iron_asset_text = (results) => {
    for (let i = 0;i + 1 < results.length; i += 2) {
      const read = reads.get(results[i]);
      if (!read)
        continue;
      reads.delete(results[i]);
      const text = results[i + 1];
      if (typeof text === "string")
        read.resolve(text);
      else
        read.reject(new Error(`couldn't read ${read.path}`));
    }
  };
  function readText(path) {
    const id = nextRead++;
    return new Promise((resolve, reject) => {
      reads.set(id, { resolve, reject, path });
      __iron_op("asset.readText", id, path);
    });
  }
  var SceneManager = {
    get sceneCount() {
      return scenes.length;
    },
    getSceneAt(index) {
      const s = scenes[index];
      if (!s)
        throw new RangeError(`no scene at ${index}`);
      return s;
    },
    getSceneByName(name) {
      return scenes.find((s) => s.name === name);
    },
    getActiveScene() {
      return active;
    },
    setActiveScene(scene) {
      if (!scenes.includes(scene))
        return false;
      setActive(scene);
      return true;
    },
    createScene(name) {
      if (scenes.some((s) => s.name === name))
        throw new Error(`a scene named ${name} is already loaded`);
      const scene = new Scene(name);
      scenes.push(scene);
      return scene;
    },
    loadScene(data, mode = "single") {
      return fromData(data, mode, null);
    },
    async loadSceneAsync(source, mode = "single") {
      if (typeof source !== "string")
        return fromData(source, mode, null);
      const data = JSON.parse(await readText(source));
      return fromData(data, mode, source);
    },
    unloadScene(scene) {
      if (!scenes.includes(scene) || scenes.length === 1)
        return false;
      unload(scene);
      if (active === scene)
        setActive(scenes[0]);
      return true;
    },
    moveGameObjectToScene(go, scene) {
      if (go.parent)
        throw new Error("only root objects move between scenes");
      if (!scene.isLoaded)
        throw new Error(`scene ${scene.name} isn't loaded`);
      go._scene = scene;
    },
    mergeScenes(source, destination) {
      for (const go of source.getRootGameObjects())
        go._scene = destination;
      SceneManager.unloadScene(source);
    },
    onSceneLoaded(f) {
      loadedListeners.add(f);
      return () => loadedListeners.delete(f);
    },
    onSceneUnloaded(f) {
      unloadedListeners.add(f);
      return () => unloadedListeners.delete(f);
    },
    onActiveSceneChanged(f) {
      activeListeners.add(f);
      return () => activeListeners.delete(f);
    }
  };
  var prefabFiles = new Map;

  class Prefab {
    data;
    src;
    constructor(data, src) {
      this.data = data;
      this.src = src;
    }
    static fromObject(go) {
      const saver = new Saver(false);
      number(saver, [go]);
      const root = saveObject(saver, go);
      if (!root)
        throw new Error("parts of a model can't be prefabs on their own; use the model's root");
      delete root.position;
      delete root.rotation;
      const data = { format: "iron.prefab", version: 1, name: go.name, root };
      if (Object.keys(saver.assets).length)
        data.assets = saver.assets;
      return new Prefab(JSON.parse(JSON.stringify(data)), null);
    }
    static fromData(data, src = null) {
      const d = data;
      if (d?.format !== "iron.prefab" || !d.root)
        throw new Error(`not prefab data${src ? `: ${src}` : ""}`);
      return new Prefab(d, src);
    }
    static load(path) {
      let p = prefabFiles.get(path);
      if (!p) {
        p = readText(path).then((text) => Prefab.fromData(JSON.parse(text), path));
        prefabFiles.set(path, p);
        p.catch(() => prefabFiles.delete(path));
      }
      return p;
    }
    get name() {
      return this.data.name;
    }
    instantiate(options = {}) {
      const root = JSON.parse(JSON.stringify(this.data.root));
      if (options.position)
        root.position = [options.position.x, options.position.y, options.position.z];
      if (options.rotation)
        root.rotation = [options.rotation.x, options.rotation.y, options.rotation.z, options.rotation.w];
      if (this.src)
        root.prefab = this.src;
      return loadObjects([root], this.data.assets, options.parent ?? null)[0];
    }
    toJSON() {
      return JSON.parse(JSON.stringify(this.data));
    }
  }
  function instantiate2(original, options = {}) {
    if (original instanceof Prefab)
      return original.instantiate(options);
    const saver = new Saver(true);
    number(saver, [original]);
    const data = saveObject(saver, original);
    if (!data)
      throw new Error("parts of a model can't be copied on their own; copy the model's root");
    if (!options.parent) {
      const t = original.transform;
      const [p, r, s] = [t.position, t.rotation, t.lossyScale];
      data.position = [p.x, p.y, p.z];
      data.rotation = [r.x, r.y, r.z, r.w];
      data.scale = [s.x, s.y, s.z];
    }
    if (options.position)
      data.position = [options.position.x, options.position.y, options.position.z];
    if (options.rotation)
      data.rotation = [options.rotation.x, options.rotation.y, options.rotation.z, options.rotation.w];
    const previous = active;
    if (!options.parent)
      active = original.scene;
    try {
      return loadObjects([data], saver.assets, options.parent ?? null)[0];
    } finally {
      active = previous;
    }
  }
  function sceneNames(data) {
    const out = [];
    const walk = (o) => {
      for (const c of o.components ?? [])
        out.push(c.type);
      for (const child of o.children ?? [])
        walk(child);
    };
    for (const o of data.objects)
      walk(o);
    return out;
  }
  function saveState() {
    const state = {
      version: 1,
      scenes: scenes.map((s) => s._serialize(true)),
      active: scenes.indexOf(active),
      keep: keep && keep.rootCount > 0 ? keep._serialize(true) : null,
      quality: _qualityState(),
      time: _timeState()
    };
    return JSON.stringify(state);
  }
  function restoreState(json) {
    const state = JSON.parse(json);
    const all = [...state.scenes, ...state.keep ? [state.keep] : []];
    const missing = [...new Set(all.flatMap(sceneNames))].filter((name) => !_classByName(name));
    if (missing.length) {
      warn(`can't keep state across the reload: ${missing.join(", ")} ${missing.length === 1 ? "isn't" : "aren't"} registered ` + `(register({ ${missing.join(", ")} }, { spawn: false })); starting over`);
      if (startScene)
        loadStartScene(startScene);
      return false;
    }
    _restoreQuality(state.quality);
    _restoreTime(state.time);
    for (const s of scenes)
      s.isLoaded = false;
    scenes = [];
    const build = (data) => {
      const scene = new Scene(data.name);
      scene._renderSettings = data.renderSettings ?? null;
      const previous = active;
      active = scene;
      try {
        loadObjects(data.objects ?? [], data.assets, null, true);
      } finally {
        active = previous;
      }
      return scene;
    };
    scenes = state.scenes.map(build);
    if (state.keep)
      keep = build(state.keep);
    if (!scenes.length)
      scenes = [new Scene("Main")];
    active = scenes[state.active] ?? scenes[0];
    if (active._renderSettings)
      applyRenderSettings(active._renderSettings);
    return true;
  }
  globalThis.__iron_save_state = saveState;
  globalThis.__iron_restore_state = restoreState;
  async function loadStartScene(path) {
    const main = active;
    try {
      const scene = await SceneManager.loadSceneAsync(path, "additive");
      SceneManager.setActiveScene(scene);
      if (main.isLoaded && main.rootCount === 0)
        SceneManager.unloadScene(main);
    } catch (e) {
      console.error(`start scene ${path}: ${e}`);
    }
  }
  var startScene = globalThis.__iron_info?.app.startScene;
  if (startScene && typeof __iron_op === "function" && !globalThis.__iron_reloading)
    loadStartScene(startScene);
  _setSceneHooks({
    active: () => active,
    dontDestroyOnLoad(go) {
      const root = go.root;
      keep ??= new Scene("DontDestroyOnLoad");
      root._scene = keep;
    },
    instantiate: instantiate2,
    reset
  });
  // ../../js/engine/src/shapes.ts
  var DEG2 = Math.PI / 180;
  // ../../js/engine/src/spline.ts
  var scratch = new Float64Array(3);
  var _init39, _base39, _dec188, _dec189, _dec190, _dec191, _dec192, _dec193;

  class SplineAnimate extends (_base39 = Component) {
    constructor() {
      super(...arguments);
      __runInitializers(_init39, 31, this);
    }
    static {
      _init39 = __decoratorStart(_base39);
      __decorateElement(_init39, 5, "container", _dec188, this);
      __decorateElement(_init39, 5, "duration", _dec189, this);
      __decorateElement(_init39, 5, "loopMode", _dec190, this);
      __decorateElement(_init39, 5, "alignment", _dec191, this);
      __decorateElement(_init39, 5, "playing", _dec192, this);
      __decorateElement(_init39, 5, "elapsed", _dec193, this);
      __decoratorMetadata(_init39, this);
    }
    spline = null;
    [(_dec188 = [
      property
    ], "container")] = __runInitializers(_init39, 8, this, null);
    [(_dec189 = [
      property
    ], "duration")] = (__runInitializers(_init39, 11, this), __runInitializers(_init39, 12, this, 5));
    [(_dec190 = [
      property
    ], "loopMode")] = (__runInitializers(_init39, 15, this), __runInitializers(_init39, 16, this, "loop"));
    [(_dec191 = [
      property
    ], "alignment")] = (__runInitializers(_init39, 19, this), __runInitializers(_init39, 20, this, "path"));
    [(_dec192 = [
      property
    ], "playing")] = (__runInitializers(_init39, 23, this), __runInitializers(_init39, 24, this, true));
    [(_dec193 = [
      property
    ], "elapsed")] = (__runInitializers(_init39, 27, this), __runInitializers(_init39, 28, this, 0));
    get t() {
      const d = Math.max(0.000001, this.duration);
      const x = this.elapsed / d;
      if (this.loopMode === "once")
        return Math.max(0, Math.min(1, x));
      if (this.loopMode === "pingPong") {
        const m = (x % 2 + 2) % 2;
        return m > 1 ? 2 - m : m;
      }
      return (x % 1 + 1) % 1;
    }
    onUpdate(dt) {
      if (this.playing)
        this.elapsed += dt;
      this.place();
    }
    place() {
      const s = this.spline;
      if (!s || s.count === 0)
        return;
      const t = this.t;
      const at = s.evaluate(t);
      let dir = s.tangent(t);
      if (this.loopMode === "pingPong" && (this.elapsed / Math.max(0.000001, this.duration) % 2 + 2) % 2 > 1)
        dir = dir.scale(-1);
      const c = this.container?.transform;
      this.transform.position = c ? c.transformPoint(at) : at;
      if (this.alignment !== "path" || dir.magnitude < 0.000000001)
        return;
      const d = c ? c.transformDirection(dir) : dir;
      this.transform.rotation = Math.abs(d.z) < 0.000001 ? Quaternion.euler(0, 0, Math.atan2(d.y, d.x) * 180 / Math.PI) : Quaternion.lookRotation(d, Vector3.up);
    }
  }
  // ../../js/engine/src/text-mesh.ts
  var _init40, _base40, _dec194, _dec195, _dec196, _dec197, _dec198, _dec199, _dec200, _dec201, _dec202;

  class TextMesh extends (_base40 = Component) {
    static {
      _init40 = __decoratorStart(_base40);
      __decorateElement(_init40, 2, "text", _dec194, this);
      __decorateElement(_init40, 2, "fontSize", _dec195, this);
      __decorateElement(_init40, 2, "color", _dec196, this);
      __decorateElement(_init40, 2, "font", _dec197, this);
      __decorateElement(_init40, 2, "fontWeight", _dec198, this);
      __decorateElement(_init40, 2, "alignment", _dec199, this);
      __decorateElement(_init40, 2, "maxWidth", _dec200, this);
      __decorateElement(_init40, 2, "pixelsPerUnit", _dec201, this);
      __decorateElement(_init40, 2, "resolution", _dec202, this);
      __decoratorMetadata(_init40, this);
    }
    _text = (__runInitializers(_init40, 5, this), "");
    _fontSize = 32;
    _color = Color.white;
    _font = null;
    _fontWeight = 400;
    _alignment = "center";
    _maxWidth = 0;
    _pixelsPerUnit = 100;
    _resolution = 2;
    started = false;
    dirty = true;
    get [(_dec194 = [
      property
    ], "text")]() {
      return this._text;
    }
    set text(t) {
      this._text = String(t);
      this.dirty = true;
    }
    get [(_dec195 = [
      property
    ], "fontSize")]() {
      return this._fontSize;
    }
    set fontSize(s) {
      this._fontSize = Math.max(1, s);
      this.dirty = true;
    }
    get [(_dec196 = [
      property
    ], "color")]() {
      return this._color;
    }
    set color(c) {
      this._color = c;
      this.dirty = true;
    }
    get [(_dec197 = [
      property
    ], "font")]() {
      return this._font;
    }
    set font(f) {
      this._font = f;
      this.dirty = true;
    }
    get [(_dec198 = [
      property
    ], "fontWeight")]() {
      return this._fontWeight;
    }
    set fontWeight(w) {
      this._fontWeight = w;
      this.dirty = true;
    }
    get [(_dec199 = [
      property
    ], "alignment")]() {
      return this._alignment;
    }
    set alignment(a) {
      this._alignment = a;
      this.dirty = true;
    }
    get [(_dec200 = [
      property
    ], "maxWidth")]() {
      return this._maxWidth;
    }
    set maxWidth(w) {
      this._maxWidth = Math.max(0, w);
      this.dirty = true;
    }
    get [(_dec201 = [
      property
    ], "pixelsPerUnit")]() {
      return this._pixelsPerUnit;
    }
    set pixelsPerUnit(p) {
      this._pixelsPerUnit = p;
      this.dirty = true;
    }
    get [(_dec202 = [
      property
    ], "resolution")]() {
      return this._resolution;
    }
    set resolution(r) {
      this._resolution = r;
      this.dirty = true;
    }
    onAwake() {
      this.started = true;
      this.send();
    }
    onPreRender() {
      if (this.dirty)
        this.send();
    }
    onDestroy() {
      __iron_op("render.remove", this.id);
    }
    send() {
      if (!this.started)
        return;
      this.dirty = false;
      const c = this._color;
      __iron_op("render.text", this.id, this.gameObject.id, this._text, [c.r, c.g, c.b, c.a], this._fontSize, this._font, this._fontWeight, this._alignment, this._maxWidth, this._pixelsPerUnit, this._resolution);
    }
  }
  // ../../js/engine/src/app.ts
  var focused = true;
  var paused = false;
  var focusListeners = new Set;
  globalThis.__iron_focus = (hasFocus, isPaused) => {
    const focusChanged = hasFocus !== focused;
    const pauseChanged = isPaused !== paused;
    focused = hasFocus;
    paused = isPaused;
    if (pauseChanged) {
      _forEachActiveComponent("onApplicationPause", (c) => c.onApplicationPause?.(isPaused));
    }
    if (focusChanged) {
      for (const f of [...focusListeners]) {
        try {
          f(hasFocus);
        } catch (e) {
          console.error(`Application.onFocusChanged: ${e}`);
        }
      }
      _forEachActiveComponent("onApplicationFocus", (c) => c.onApplicationFocus?.(hasFocus));
    }
  };
  // ../../js/engine/src/time.ts
  var MS_PER_SECOND = 1000;
  var MS_PER_MINUTE = 60000;
  var MS_PER_HOUR = 3600000;
  var MS_PER_DAY = 86400000;

  class TimeSpan {
    totalMilliseconds;
    constructor(milliseconds) {
      this.totalMilliseconds = Math.round(milliseconds);
    }
    static zero = new TimeSpan(0);
    static of(days, hours = 0, minutes = 0, seconds = 0, milliseconds = 0) {
      return new TimeSpan(days * MS_PER_DAY + hours * MS_PER_HOUR + minutes * MS_PER_MINUTE + seconds * MS_PER_SECOND + milliseconds);
    }
    static fromDays(d) {
      return new TimeSpan(d * MS_PER_DAY);
    }
    static fromHours(h) {
      return new TimeSpan(h * MS_PER_HOUR);
    }
    static fromMinutes(m) {
      return new TimeSpan(m * MS_PER_MINUTE);
    }
    static fromSeconds(s) {
      return new TimeSpan(s * MS_PER_SECOND);
    }
    static fromMilliseconds(ms) {
      return new TimeSpan(ms);
    }
    static parse(text) {
      const m = /^\s*(-)?(?:(\d+)\.)?(\d{1,2}):(\d{1,2})(?::(\d{1,2})(?:\.(\d{1,7}))?)?\s*$/.exec(text);
      if (!m)
        throw new Error(`not a TimeSpan: "${text}"`);
      const [, neg, d, h, min, s, frac] = m;
      if (Number(h) > 23 || Number(min) > 59 || Number(s ?? 0) > 59)
        throw new Error(`TimeSpan out of range: "${text}"`);
      const ms = frac ? Number(frac.padEnd(3, "0").slice(0, 3)) : 0;
      const span = TimeSpan.of(Number(d ?? 0), Number(h), Number(min), Number(s ?? 0), ms);
      return neg ? span.negate() : span;
    }
    part(unit, mod) {
      return Math.trunc(this.totalMilliseconds / unit) % mod;
    }
    get days() {
      return Math.trunc(this.totalMilliseconds / MS_PER_DAY);
    }
    get hours() {
      return this.part(MS_PER_HOUR, 24);
    }
    get minutes() {
      return this.part(MS_PER_MINUTE, 60);
    }
    get seconds() {
      return this.part(MS_PER_SECOND, 60);
    }
    get milliseconds() {
      return this.totalMilliseconds % 1000;
    }
    get totalDays() {
      return this.totalMilliseconds / MS_PER_DAY;
    }
    get totalHours() {
      return this.totalMilliseconds / MS_PER_HOUR;
    }
    get totalMinutes() {
      return this.totalMilliseconds / MS_PER_MINUTE;
    }
    get totalSeconds() {
      return this.totalMilliseconds / MS_PER_SECOND;
    }
    add(t) {
      return new TimeSpan(this.totalMilliseconds + t.totalMilliseconds);
    }
    subtract(t) {
      return new TimeSpan(this.totalMilliseconds - t.totalMilliseconds);
    }
    multiply(factor) {
      return new TimeSpan(this.totalMilliseconds * factor);
    }
    divide(divisor) {
      return new TimeSpan(this.totalMilliseconds / divisor);
    }
    negate() {
      return new TimeSpan(-this.totalMilliseconds);
    }
    duration() {
      return new TimeSpan(Math.abs(this.totalMilliseconds));
    }
    compareTo(t) {
      return Math.sign(this.totalMilliseconds - t.totalMilliseconds);
    }
    equals(t) {
      return this.totalMilliseconds === t.totalMilliseconds;
    }
    toString(format) {
      const abs = this.duration();
      const pad = (n, w = 2) => String(n).padStart(w, "0");
      if (!format || format === "c") {
        const sign = this.totalMilliseconds < 0 ? "-" : "";
        const day = abs.days ? `${abs.days}.` : "";
        const frac = abs.milliseconds ? `.${pad(abs.milliseconds, 3)}` : "";
        return `${sign}${day}${pad(abs.hours)}:${pad(abs.minutes)}:${pad(abs.seconds)}${frac}`;
      }
      return formatTokens(format, (token) => {
        const c = token[0];
        const n = token.length;
        switch (c) {
          case "d":
            return pad(abs.days, n);
          case "h":
            return pad(abs.hours, Math.min(n, 2));
          case "m":
            return pad(abs.minutes, Math.min(n, 2));
          case "s":
            return pad(abs.seconds, Math.min(n, 2));
          case "f":
          case "F":
            return fraction(abs.milliseconds, n, c === "F");
          default:
            return;
        }
      });
    }
  }
  function fraction(ms, digits, trim) {
    let s = String(ms).padStart(3, "0").padEnd(digits, "0").slice(0, digits);
    if (trim)
      s = s.replace(/0+$/, "");
    return s;
  }
  function formatTokens(format, render) {
    let out = "";
    for (let i = 0;i < format.length; ) {
      const c = format[i];
      if (c === "\\" && i + 1 < format.length) {
        out += format[i + 1];
        i += 2;
        continue;
      }
      if (c === "'" || c === '"') {
        const end = format.indexOf(c, i + 1);
        const stop = end < 0 ? format.length : end;
        out += format.slice(i + 1, stop);
        i = stop + 1;
        continue;
      }
      let j = i;
      while (j < format.length && format[j] === c)
        j++;
      const token = format.slice(i, j);
      const rendered = /[a-zA-Z]/.test(c) ? render(token) : undefined;
      out += rendered ?? token;
      i = j;
    }
    return out;
  }
  // ../../js/engine/src/random.ts
  class Rng {
    state;
    constructor(seed = (Date.now() ^ Math.random() * 4294967296) >>> 0) {
      this.state = seed >>> 0;
    }
    nextDouble() {
      this.state = this.state + 1831565813 >>> 0;
      let t = this.state;
      t = Math.imul(t ^ t >>> 15, t | 1);
      t ^= t + Math.imul(t ^ t >>> 7, t | 61);
      return ((t ^ t >>> 14) >>> 0) / 4294967296;
    }
    next(minOrMax, max) {
      const [lo, hi] = max === undefined ? [0, minOrMax ?? 2147483647] : [minOrMax ?? 0, max];
      if (hi <= lo)
        return lo;
      return lo + Math.floor(this.nextDouble() * (hi - lo));
    }
    float(min, max) {
      return min + this.nextDouble() * (max - min);
    }
    int(min, max) {
      return this.next(Math.ceil(min), Math.floor(max));
    }
    bool(chance = 0.5) {
      return this.nextDouble() < chance;
    }
    pick(items) {
      return items.length ? items[this.next(items.length)] : undefined;
    }
    shuffle(items) {
      for (let i = items.length - 1;i > 0; i--) {
        const j = this.next(i + 1);
        [items[i], items[j]] = [items[j], items[i]];
      }
      return items;
    }
    insideUnitCircle() {
      const angle = this.nextDouble() * Math.PI * 2;
      const r = Math.sqrt(this.nextDouble());
      return new Vector2(Math.cos(angle) * r, Math.sin(angle) * r);
    }
    onUnitSphere() {
      const z = this.nextDouble() * 2 - 1;
      const angle = this.nextDouble() * Math.PI * 2;
      const r = Math.sqrt(1 - z * z);
      return new Vector3(Math.cos(angle) * r, Math.sin(angle) * r, z);
    }
    insideUnitSphere() {
      return this.onUnitSphere().scale(Math.cbrt(this.nextDouble()));
    }
    rotation() {
      const [u1, u2, u3] = [this.nextDouble(), this.nextDouble(), this.nextDouble()];
      const a = Math.sqrt(1 - u1);
      const b = Math.sqrt(u1);
      return new Quaternion(a * Math.sin(2 * Math.PI * u2), a * Math.cos(2 * Math.PI * u2), b * Math.sin(2 * Math.PI * u3), b * Math.cos(2 * Math.PI * u3));
    }
  }
  var shared = new Rng;
  // ../../js/engine/src/gizmos.ts
  var DEG3 = Math.PI / 180;
  var CIRCLE = 48;
  var lines = [];
  function push(a, b) {
    const m = Gizmos.matrix;
    const p = m.multiplyPoint3x4(a);
    const q = m.multiplyPoint3x4(b);
    const c = Gizmos.color;
    lines.push(p.x, p.y, p.z, q.x, q.y, q.z, c.r, c.g, c.b, c.a);
  }
  function basis(normal) {
    const n = normal.normalized;
    const u = n.cross(Math.abs(n.y) < 0.9 ? Vector3.up : Vector3.right).normalized;
    return [u, n.cross(u)];
  }
  var Gizmos = {
    color: Color.white,
    matrix: Matrix4x4.identity,
    drawLine(from, to) {
      push(Vector3.from(from), Vector3.from(to));
    },
    drawRay(from, direction) {
      const f = Vector3.from(from);
      push(f, f.add(direction));
    },
    drawWireCube(center, size) {
      const c = Vector3.from(center);
      const h = Vector3.from(size).scale(0.5);
      const corner = (i) => c.add(new Vector3(i & 1 ? h.x : -h.x, i & 2 ? h.y : -h.y, i & 4 ? h.z : -h.z));
      for (let i = 0;i < 8; i++) {
        for (const bit of [1, 2, 4]) {
          if ((i & bit) === 0)
            push(corner(i), corner(i | bit));
        }
      }
    },
    drawWireDisc(center, normal, radius) {
      const c = Vector3.from(center);
      const [u, v] = basis(Vector3.from(normal));
      let prev = c.add(u.scale(radius));
      for (let i = 1;i <= CIRCLE; i++) {
        const a = i / CIRCLE * Math.PI * 2;
        const p = c.add(u.scale(Math.cos(a) * radius)).add(v.scale(Math.sin(a) * radius));
        push(prev, p);
        prev = p;
      }
    },
    drawWireSphere(center, radius) {
      for (const n of [Vector3.right, Vector3.up, Vector3.back])
        Gizmos.drawWireDisc(center, n, radius);
    },
    drawFrustum(center, fov, maxRange, minRange, aspect) {
      const c = Vector3.from(center);
      const t = Math.tan(fov / 2 * DEG3);
      const rect = (d) => {
        const h = d * t;
        const w = h * aspect;
        return [new Vector3(-w, -h, -d), new Vector3(w, -h, -d), new Vector3(w, h, -d), new Vector3(-w, h, -d)].map((p) => c.add(p));
      };
      const near = rect(minRange);
      const far = rect(maxRange);
      for (let i = 0;i < 4; i++) {
        push(near[i], near[(i + 1) % 4]);
        push(far[i], far[(i + 1) % 4]);
        push(near[i], far[i]);
      }
    },
    get lineCount() {
      return lines.length / 10;
    },
    clear() {
      lines = [];
    },
    present(camera) {
      __iron_op("render.gizmos", camera.id, lines);
      lines = [];
    }
  };
  // ../../js/engine/src/primitives.ts
  function createPrimitive(type, options = {}) {
    const name = options.name ?? type[0].toUpperCase() + type.slice(1);
    const go = new GameObject(name, options);
    go.addComponent(MeshRenderer, { mesh: type });
    switch (type) {
      case "cube":
        go.addComponent(BoxCollider);
        break;
      case "sphere":
        go.addComponent(SphereCollider);
        break;
      case "capsule":
        go.addComponent(CapsuleCollider);
        break;
      case "cylinder":
        go.addComponent(CapsuleCollider);
        break;
      case "plane":
        go.addComponent(BoxCollider, { size: new Vector3(10, 0.01, 10) });
        break;
      case "quad":
        go.addComponent(MeshCollider, {
          points: [new Vector3(-0.5, -0.5, 0), new Vector3(0.5, -0.5, 0), new Vector3(0.5, 0.5, 0), new Vector3(-0.5, 0.5, 0)],
          indices: [0, 1, 2, 0, 2, 3]
        });
        break;
    }
    return go;
  }
  setPrimitiveFactory(createPrimitive);

  // ../../js/engine/src/index.ts
  function op(name, ...args) {
    __iron_op(name, ...args);
  }

  // ../../js/react/src/index.ts
  var import_react2 = __toESM(require_react(), 1);
  var import_constants3 = __toESM(require_constants(), 1);

  // ../../js/react/src/events.ts
  var import_constants2 = __toESM(require_constants(), 1);

  // ../../js/react/src/host.ts
  var import_react = __toESM(require_react(), 1);
  var import_react_reconciler = __toESM(require_react_reconciler(), 1);
  var import_constants = __toESM(require_constants(), 1);

  // ../../js/react/src/opcodes.ts
  var CREATE = 0;
  var SET_STYLE = 1;
  var APPEND_CHILD = 2;
  var INSERT_BEFORE = 3;
  var REMOVE_CHILD = 4;
  var ROOT_ID = 0;
  var CREATE_TEXT = 5;
  var SET_TEXT = 6;
  var SET_ATTRS = 7;

  // ../../js/react/src/attrs.ts
  var ATTRS = new Set([
    "className",
    "id",
    "disabled",
    "checked",
    "draggable",
    "tabIndex",
    "dir",
    "value",
    "defaultValue",
    "placeholder",
    "readOnly",
    "rows",
    "src",
    "open",
    "modal",
    "closedBy"
  ]);
  var SVG_TYPES = new Set([
    "svg",
    "g",
    "rect",
    "circle",
    "ellipse",
    "line",
    "polyline",
    "polygon",
    "path",
    "tspan",
    "defs",
    "use",
    "symbol",
    "linearGradient",
    "radialGradient",
    "stop",
    "clipPath",
    "mask",
    "pattern",
    "title"
  ]);
  var NOT_ATTRS = new Set(["children", "style", "key", "ref"]);
  function isAttrOf(name, value, type) {
    if (isAttr(name))
      return true;
    if (!SVG_TYPES.has(type) && type !== "text")
      return false;
    if (NOT_ATTRS.has(name) || /^on[A-Z]/.test(name))
      return false;
    return value === null || ["string", "number", "boolean"].includes(typeof value);
  }
  function isAttr(name) {
    return ATTRS.has(name) || name.startsWith("data-");
  }
  function attrEntries(props, type = "box") {
    const out = [];
    for (const [name, value] of Object.entries(props)) {
      if (value !== undefined && isAttrOf(name, value, type))
        out.push(name, value);
    }
    return out;
  }
  function attrDiff(prev, next, type = "box") {
    const out = [];
    for (const [name, value] of Object.entries(next)) {
      if (value !== undefined && value !== prev[name] && isAttrOf(name, value, type)) {
        out.push(name, value);
      }
    }
    for (const [name, value] of Object.entries(prev)) {
      if (value !== undefined && next[name] === undefined && isAttrOf(name, value, type))
        out.push(name, null);
    }
    return out;
  }

  // ../../js/react/src/style.ts
  function styleEntries(style) {
    const out = [];
    if (!style)
      return out;
    for (const [prop, value] of Object.entries(style)) {
      if (value !== undefined)
        out.push(prop, value);
    }
    return out;
  }
  function styleDiff(prev, next) {
    const out = [];
    const a = prev ?? {};
    const b = next ?? {};
    for (const [prop, value] of Object.entries(b)) {
      if (value !== undefined && value !== a[prop])
        out.push(prop, value);
    }
    for (const [prop, value] of Object.entries(a)) {
      if (value !== undefined && b[prop] === undefined)
        out.push(prop, null);
    }
    return out;
  }

  // ../../js/react/src/host.ts
  class Instance {
    id;
    type;
    props;
    constructor(id, type, props) {
      this.id = id;
      this.type = type;
      this.props = props;
      const p = props;
      if (type === "input" || type === "textarea")
        this.value = String(p.value ?? p.defaultValue ?? "");
    }
    value;
    focus() {
      op("focus", this.id);
    }
    blur() {
      op("blur", this.id);
    }
    setPointerCapture(_pointerId) {
      op("setPointerCapture", this.id);
    }
    releasePointerCapture(_pointerId) {
      op("releasePointerCapture", this.id);
    }
    hasPointerCapture(_pointerId) {
      return pointerCapture.element === this;
    }
    scrollTo(left, top) {
      op("scrollTo", this.id, left, top);
    }
    scrollBy(dx, dy) {
      op("scrollBy", this.id, dx, dy);
    }
    scrollIntoView() {
      op("scrollIntoView", this.id);
    }
  }
  var pointerCapture = { element: null };
  var instances2 = new Map;
  var screenContainer = { id: ROOT_ID, target: 0, children: new Set, pending: [] };
  var containers = new Map([[0, screenContainer]]);
  function containerFor(target) {
    let c = containers.get(target);
    if (!c) {
      c = { id: ROOT_ID, target, children: new Set, pending: [] };
      containers.set(target, c);
    }
    return c;
  }
  var pending2 = screenContainer.pending;
  var owners = new Map;
  function queueFor(id) {
    return (owners.get(id) ?? screenContainer).pending;
  }
  var nextId = 1;
  function flushContainer(c) {
    if (c.pending.length === 0)
      return;
    const ops = c.pending.splice(0, c.pending.length);
    if (c.target === 0)
      __iron_commit(ops);
    else
      __iron_commit(ops, c.target);
  }
  function flush() {
    for (const c of containers.values())
      flushContainer(c);
  }
  var ELEMENT_TYPES = new Set([
    "box",
    "text",
    "span",
    "input",
    "textarea",
    "image",
    "list",
    "dialog",
    ...SVG_TYPES
  ]);
  var currentUpdatePriority = import_constants.NoEventPriority;
  var HostTransitionContext = import_react.createContext(null);
  var reconciler = import_react_reconciler.default({
    supportsMutation: true,
    supportsPersistence: false,
    supportsHydration: false,
    isPrimaryRenderer: true,
    supportsMicrotasks: true,
    scheduleMicrotask: (fn) => queueMicrotask(() => void fn()),
    scheduleTimeout: (fn, delay) => setTimeout(fn, delay),
    cancelTimeout: (id) => clearTimeout(id),
    noTimeout: -1,
    rendererVersion: "0.1.0",
    rendererPackageName: "@iron/react",
    extraDevToolsConfig: null,
    bindToConsole(methodName, args, badgeName) {
      const log = console[methodName] ?? console.log;
      return log.bind(console, `[${badgeName}]`, ...args);
    },
    createInstance(type, props, root) {
      if (!ELEMENT_TYPES.has(type)) {
        throw new Error(`<${type}> is not an Iron element; available: ${[...ELEMENT_TYPES].join(", ")}`);
      }
      const id = nextId++;
      owners.set(id, root);
      const pending = root.pending;
      pending.push([CREATE, id, type]);
      const entries = styleEntries(props.style);
      if (entries.length > 0)
        pending.push([SET_STYLE, id, ...entries]);
      const attrs = attrEntries(props, type);
      if (attrs.length > 0)
        pending.push([SET_ATTRS, id, ...attrs]);
      const instance = new Instance(id, type, props);
      instances2.set(id, instance);
      return instance;
    },
    createTextInstance(text, root) {
      const id = nextId++;
      owners.set(id, root);
      root.pending.push([CREATE_TEXT, id, text]);
      return { id, type: "#text", text };
    },
    commitTextUpdate(instance, _oldText, newText) {
      instance.text = newText;
      queueFor(instance.id).push([SET_TEXT, instance.id, newText]);
    },
    appendInitialChild(parent, child) {
      queueFor(parent.id).push([APPEND_CHILD, parent.id, child.id]);
    },
    appendChild(parent, child) {
      queueFor(parent.id).push([APPEND_CHILD, parent.id, child.id]);
    },
    appendChildToContainer(container, child) {
      container.children.add(child.id);
      container.pending.push([APPEND_CHILD, ROOT_ID, child.id]);
    },
    insertBefore(parent, child, before) {
      queueFor(parent.id).push([INSERT_BEFORE, parent.id, child.id, before.id]);
    },
    insertInContainerBefore(container, child, before) {
      container.children.add(child.id);
      container.pending.push([INSERT_BEFORE, ROOT_ID, child.id, before.id]);
    },
    removeChild(parent, child) {
      queueFor(parent.id).push([REMOVE_CHILD, parent.id, child.id]);
    },
    removeChildFromContainer(container, child) {
      container.children.delete(child.id);
      container.pending.push([REMOVE_CHILD, ROOT_ID, child.id]);
    },
    clearContainer(container) {
      for (const id of container.children)
        container.pending.push([REMOVE_CHILD, ROOT_ID, id]);
      container.children.clear();
    },
    commitUpdate(instance, type, prevProps, nextProps) {
      const value = nextProps.value;
      if (value !== undefined && value !== null)
        instance.value = String(value);
      const changes = styleDiff(prevProps.style, nextProps.style);
      if (changes.length > 0)
        queueFor(instance.id).push([SET_STYLE, instance.id, ...changes]);
      const attrs = attrDiff(prevProps, nextProps, type);
      if (attrs.length > 0)
        queueFor(instance.id).push([SET_ATTRS, instance.id, ...attrs]);
      instance.props = nextProps;
    },
    hideInstance(instance) {
      queueFor(instance.id).push([SET_STYLE, instance.id, "display", "none"]);
    },
    unhideInstance(instance, props) {
      queueFor(instance.id).push([SET_STYLE, instance.id, "display", props.style?.display ?? null]);
    },
    hideTextInstance(instance) {
      queueFor(instance.id).push([SET_TEXT, instance.id, ""]);
    },
    unhideTextInstance(instance, text) {
      instance.text = text;
      queueFor(instance.id).push([SET_TEXT, instance.id, text]);
    },
    finalizeInitialChildren: () => false,
    shouldSetTextContent: () => false,
    getRootHostContext: () => ({}),
    getChildHostContext: (parent) => parent,
    getPublicInstance: (instance) => instance,
    prepareForCommit: () => null,
    resetAfterCommit: (container) => flushContainer(container),
    preparePortalMount: () => {},
    detachDeletedInstance(node) {
      instances2.delete(node.id);
      owners.delete(node.id);
    },
    getInstanceFromNode: () => null,
    beforeActiveInstanceBlur: () => {},
    afterActiveInstanceBlur: () => {},
    prepareScopeUpdate: () => {},
    getInstanceFromScope: () => null,
    setCurrentUpdatePriority(priority) {
      currentUpdatePriority = priority;
    },
    getCurrentUpdatePriority: () => currentUpdatePriority,
    resolveUpdatePriority: () => currentUpdatePriority !== import_constants.NoEventPriority ? currentUpdatePriority : import_constants.DefaultEventPriority,
    NotPendingTransition: null,
    HostTransitionContext,
    resetFormInstance: () => {},
    shouldAttemptEagerTransition: () => false,
    trackSchedulerEvent: () => {},
    resolveEventType: () => null,
    resolveEventTimeStamp: () => -1.1,
    requestPostPaintCallback: () => {},
    maySuspendCommit: () => false,
    maySuspendCommitOnUpdate: () => false,
    maySuspendCommitInSyncRender: () => false,
    preloadInstance: () => true,
    startSuspendingCommit: () => null,
    suspendInstance: () => {},
    suspendOnActiveViewTransition: () => {},
    waitForCommitToBeReady: () => null,
    getSuspendedCommitReason: () => null
  });
  reconciler.injectIntoDevTools();
  function withUpdatePriority(priority, fn) {
    const previous = currentUpdatePriority;
    currentUpdatePriority = priority;
    try {
      fn();
    } finally {
      currentUpdatePriority = previous;
    }
  }

  // ../../js/react/src/events.ts
  var HANDLERS = {
    pointermove: ["onPointerMove", "onMouseMove"],
    pointerdown: ["onPointerDown", "onMouseDown"],
    pointerup: ["onPointerUp", "onMouseUp"],
    pointerenter: ["onPointerEnter", "onMouseEnter"],
    pointerleave: ["onPointerLeave", "onMouseLeave"],
    click: ["onClick"],
    dblclick: ["onDoubleClick"],
    contextmenu: ["onContextMenu"],
    wheel: ["onWheel"],
    keydown: ["onKeyDown"],
    keyup: ["onKeyUp"],
    focusin: ["onFocus"],
    focusout: ["onBlur"],
    scroll: ["onScroll"],
    input: ["onChange", "onInput"],
    cancel: ["onCancel"],
    close: ["onClose"],
    transitionrun: ["onTransitionRun"],
    transitionstart: ["onTransitionStart"],
    transitionend: ["onTransitionEnd"],
    transitioncancel: ["onTransitionCancel"],
    animationstart: ["onAnimationStart"],
    animationiteration: ["onAnimationIteration"],
    animationend: ["onAnimationEnd"],
    animationcancel: ["onAnimationCancel"],
    gotpointercapture: ["onGotPointerCapture"],
    lostpointercapture: ["onLostPointerCapture"],
    dragstart: ["onDragStart"],
    drag: ["onDrag"],
    dragenter: ["onDragEnter"],
    dragover: ["onDragOver"],
    dragleave: ["onDragLeave"],
    drop: ["onDrop"],
    dragend: ["onDragEnd"]
  };
  var NON_BUBBLING = new Set(["pointerenter", "pointerleave", "scroll", "cancel", "close"]);
  var CONTINUOUS = new Set(["pointermove", "pointerenter", "pointerleave", "wheel", "scroll", "drag", "dragover"]);

  class IronEvent {
    type;
    target;
    bubbles;
    eventPhase = 0;
    currentTarget = null;
    defaultPrevented = false;
    stopped = false;
    constructor(type, target, bubbles) {
      this.type = type;
      this.target = target;
      this.bubbles = bubbles;
    }
    preventDefault() {
      this.defaultPrevented = true;
    }
    stopPropagation() {
      this.stopped = true;
    }
    isDefaultPrevented() {
      return this.defaultPrevented;
    }
    isPropagationStopped() {
      return this.stopped;
    }
  }

  class IronPointerEvent extends IronEvent {
    clientX;
    clientY;
    button;
    buttons;
    detail;
    shiftKey;
    ctrlKey;
    altKey;
    metaKey;
    pointerType = "mouse";
    pointerId = 1;
    isPrimary = true;
    constructor(type, target, bubbles, data) {
      super(type, target, bubbles);
      const [x = 0, y = 0, button = 0, buttons = 0, detail = 0, , , mods = 0] = data;
      this.clientX = x;
      this.clientY = y;
      this.button = button;
      this.buttons = buttons;
      this.detail = detail;
      const m = modifiers(mods);
      this.shiftKey = m.shiftKey;
      this.ctrlKey = m.ctrlKey;
      this.altKey = m.altKey;
      this.metaKey = m.metaKey;
    }
    get x() {
      return this.clientX;
    }
    get y() {
      return this.clientY;
    }
  }

  class IronWheelEvent extends IronPointerEvent {
    deltaX;
    deltaY;
    deltaZ = 0;
    deltaMode = 0;
    constructor(type, target, bubbles, data) {
      super(type, target, bubbles, data);
      this.deltaX = data[5] ?? 0;
      this.deltaY = data[6] ?? 0;
    }
  }
  function modifiers(bits) {
    return { shiftKey: (bits & 1) !== 0, ctrlKey: (bits & 2) !== 0, altKey: (bits & 4) !== 0, metaKey: (bits & 8) !== 0 };
  }

  class IronKeyboardEvent extends IronEvent {
    key;
    code;
    repeat;
    shiftKey;
    ctrlKey;
    altKey;
    metaKey;
    constructor(type, target, data, strings) {
      super(type, target, true);
      this.key = strings[0] ?? "";
      this.code = strings[1] ?? "";
      this.repeat = data[8] === 1;
      const m = modifiers(data[7] ?? 0);
      this.shiftKey = m.shiftKey;
      this.ctrlKey = m.ctrlKey;
      this.altKey = m.altKey;
      this.metaKey = m.metaKey;
    }
  }

  class IronFocusEvent extends IronEvent {
    relatedTarget;
    constructor(type, target, data) {
      super(type, target, true);
      this.relatedTarget = instances2.get(data[9] ?? 0) ?? null;
    }
  }

  class IronInputEvent extends IronEvent {
    value;
    constructor(type, target, strings) {
      super(type, target, true);
      this.value = strings[0] ?? "";
    }
  }

  class IronTransitionEvent extends IronEvent {
    propertyName;
    elapsedTime;
    constructor(type, target, data, strings) {
      super(type, target, true);
      this.propertyName = strings[0] ?? "";
      this.elapsedTime = data[5] ?? 0;
    }
  }

  class IronAnimationEvent extends IronEvent {
    animationName;
    elapsedTime;
    constructor(type, target, data, strings) {
      super(type, target, true);
      this.animationName = strings[0] ?? "";
      this.elapsedTime = data[5] ?? 0;
    }
  }

  class IronDataTransfer {
    dropEffect = "none";
    effectAllowed = "uninitialized";
    files;
    data = new Map;
    constructor(files = []) {
      this.files = files.map((path) => ({ path, name: path.split(/[\\/]/).filter(Boolean).pop() ?? path }));
    }
    get types() {
      const types = [...this.data.keys()];
      if (this.files.length)
        types.push("Files");
      return types;
    }
    setData(format, data) {
      this.data.set(format.toLowerCase(), String(data));
    }
    getData(format) {
      return this.data.get(format.toLowerCase()) ?? "";
    }
    clearData(format) {
      if (format === undefined)
        this.data.clear();
      else
        this.data.delete(format.toLowerCase());
    }
    setDragImage(element, x = 0, y = 0) {
      op("dragImage", element?.id ?? 0, x, y);
    }
  }
  var elementDrag = null;
  var fileDrag = null;
  function transferFor(type, files) {
    if (files.length) {
      const key = files.join(`
`);
      if (fileDrag?.key !== key)
        fileDrag = { key, transfer: new IronDataTransfer(files) };
      return fileDrag.transfer;
    }
    if (type === "dragstart" || !elementDrag)
      elementDrag = new IronDataTransfer;
    return elementDrag;
  }

  class IronDragEvent extends IronPointerEvent {
    dataTransfer;
    dropped;
    constructor(type, target, data, transfer) {
      super(type, target, true, data);
      this.dataTransfer = transfer;
      this.dropped = type === "dragend" && data[4] === 1;
      if (type === "dragend" && !this.dropped)
        transfer.dropEffect = "none";
    }
  }

  class IronScrollEvent extends IronEvent {
    scrollLeft;
    scrollTop;
    scrollWidth;
    scrollHeight;
    clientWidth;
    clientHeight;
    constructor(type, target, data) {
      super(type, target, false);
      this.scrollLeft = data[10] ?? 0;
      this.scrollTop = data[11] ?? 0;
      this.scrollWidth = data[12] ?? 0;
      this.scrollHeight = data[13] ?? 0;
      this.clientWidth = data[14] ?? 0;
      this.clientHeight = data[15] ?? 0;
    }
  }
  function invoke(node, names, event, phase) {
    const props = node.props;
    for (const name of names) {
      const handler = props[name];
      if (typeof handler !== "function")
        continue;
      event.currentTarget = node;
      event.eventPhase = phase;
      try {
        handler(event);
      } catch (e) {
        const err = e instanceof Error ? e : new Error(String(e));
        console.error(`[react] error in ${name}: ${err.message}${err.stack ? `
${err.stack}` : ""}`);
      }
    }
  }
  function createEvent(type, target, bubbles, data, strings) {
    switch (type) {
      case "wheel":
        return new IronWheelEvent(type, target, bubbles, data);
      case "keydown":
      case "keyup":
        return new IronKeyboardEvent(type, target, data, strings);
      case "focusin":
      case "focusout":
        return new IronFocusEvent(type, target, data);
      case "scroll":
        return new IronScrollEvent(type, target, data);
      case "input":
        return new IronInputEvent(type, target, strings);
      case "transitionrun":
      case "transitionstart":
      case "transitionend":
      case "transitioncancel":
        return new IronTransitionEvent(type, target, data, strings);
      case "animationstart":
      case "animationiteration":
      case "animationend":
      case "animationcancel":
        return new IronAnimationEvent(type, target, data, strings);
      case "dragstart":
      case "drag":
      case "dragenter":
      case "dragover":
      case "dragleave":
      case "drop":
      case "dragend":
        return new IronDragEvent(type, target, data, transferFor(type, strings));
      default:
        return new IronPointerEvent(type, target, bubbles, data);
    }
  }
  function dispatchUiEvent(type, path, data, strings = []) {
    if (type === "gotpointercapture")
      pointerCapture.element = instances2.get(path[0] ?? -1) ?? null;
    if (type === "lostpointercapture")
      pointerCapture.element = null;
    const names = HANDLERS[type];
    const target = instances2.get(path[0] ?? -1);
    if (!names || !target) {
      if (type === "dragend")
        elementDrag = null;
      return false;
    }
    const nodes = path.map((id) => instances2.get(id)).filter((n) => n !== undefined);
    const bubbles = !NON_BUBBLING.has(type);
    const event = createEvent(type, target, bubbles, data, strings);
    if (event instanceof IronInputEvent)
      target.value = event.value;
    const capture = names.map((n) => `${n}Capture`);
    const priority = CONTINUOUS.has(type) ? import_constants2.ContinuousEventPriority : import_constants2.DiscreteEventPriority;
    withUpdatePriority(priority, () => {
      for (let i = nodes.length - 1;i >= 0 && !event.isPropagationStopped(); i--) {
        invoke(nodes[i], capture, event, i === 0 ? 2 : 1);
      }
      const last = bubbles ? nodes.length : 1;
      for (let i = 0;i < last && !event.isPropagationStopped(); i++) {
        invoke(nodes[i], names, event, i === 0 ? 2 : 3);
      }
    });
    event.currentTarget = null;
    event.eventPhase = 0;
    if (type === "dragend")
      elementDrag = null;
    reconciler.flushSyncWork();
    if (event instanceof IronInputEvent)
      restoreControlledValue(target);
    return event.defaultPrevented;
  }
  function restoreControlledValue(target) {
    const controlled = target.props.value;
    if (controlled === undefined || controlled === null)
      return;
    const value = String(controlled);
    if (target.value === value)
      return;
    target.value = value;
    queueFor(target.id).push([SET_ATTRS, target.id, "value", value]);
    flush();
  }
  globalThis.__iron_ui_event = dispatchUiEvent;

  // ../../js/react/src/index.ts
  function Text({ children, ...props }) {
    return import_react2.createElement("text", props, children);
  }
  var rootsCreated = new Set;
  var roots2 = new Map;
  var reportError2 = (kind) => (error) => {
    const e = error instanceof Error ? error : new Error(String(error));
    console.error(`[react] ${kind}: ${e.message}${e.stack ? `
${e.stack}` : ""}`);
  };
  function createRoot(target) {
    const id = typeof target === "number" ? target : target?.id ?? 0;
    const existing = roots2.get(id);
    if (existing && _loadingGame())
      return existing;
    if (rootsCreated.has(id)) {
      throw new Error(id === 0 ? "createRoot() can only be called once per game" : `panel ${id} already has a root`);
    }
    rootsCreated.add(id);
    const container = id === 0 ? screenContainer : containerFor(id);
    let rendered = false;
    const root = reconciler.createContainer(container, import_constants3.ConcurrentRoot, null, false, null, "iron", reportError2("uncaught error"), reportError2("caught error"), reportError2("recoverable error"), () => {}, null);
    const handle = {
      render(element) {
        if (_loadingGame() && rendered)
          return;
        rendered = true;
        reconciler.updateContainerSync(element, root, null, null);
        reconciler.flushSyncWork();
      },
      unmount() {
        reconciler.updateContainerSync(null, root, null, null);
        reconciler.flushSyncWork();
        rootsCreated.delete(id);
        roots2.delete(id);
      }
    };
    roots2.set(id, handle);
    return handle;
  }

  // src/render_error.tsx
  var import_react4 = __toESM(require_react(), 1);
  var jsx_runtime = __toESM(require_jsx_runtime(), 1);
  var bump = () => {};
  function Fragile(props) {
    if (props.n > 0)
      throw new Error("render failed on purpose");
    return /* @__PURE__ */ jsx_runtime.jsxs("box", {
      style: { padding: 8, flexDirection: "column", gap: 4 },
      children: [
        /* @__PURE__ */ jsx_runtime.jsx(Text, {
          children: `count ${props.n}`
        }),
        /* @__PURE__ */ jsx_runtime.jsxs("box", {
          style: { flexDirection: "row", gap: 4 },
          children: [
            /* @__PURE__ */ jsx_runtime.jsx(Text, {
              children: "a"
            }),
            /* @__PURE__ */ jsx_runtime.jsx(Text, {
              children: "b"
            })
          ]
        }),
        /* @__PURE__ */ jsx_runtime.jsx("box", {
          style: { position: "absolute", left: 4, top: 4, width: 20, height: 10 },
          children: /* @__PURE__ */ jsx_runtime.jsx(Text, {
            children: "pin"
          })
        })
      ]
    });
  }
  function App() {
    const [n, setN] = import_react4.useState(0);
    bump = () => setN((x) => x + 1);
    return /* @__PURE__ */ jsx_runtime.jsxs("box", {
      style: { width: "100%", height: "100%", flexDirection: "column" },
      children: [
        /* @__PURE__ */ jsx_runtime.jsx(Text, {
          children: "header"
        }),
        /* @__PURE__ */ jsx_runtime.jsx(Fragile, {
          n
        }),
        /* @__PURE__ */ jsx_runtime.jsx(Text, {
          children: "footer"
        }),
        /* @__PURE__ */ jsx_runtime.jsx("box", {
          style: { position: "absolute", left: 0, top: 0, width: "100%", height: "100%" },
          children: /* @__PURE__ */ jsx_runtime.jsx("box", {
            style: { position: "absolute", right: 10, bottom: 10, width: 50, height: 20 },
            children: /* @__PURE__ */ jsx_runtime.jsx(Text, {
              children: "overlay"
            })
          })
        })
      ]
    });
  }
  createRoot().render(/* @__PURE__ */ jsx_runtime.jsx(App, {}));
  globalThis.__bump = () => bump();
})();
