export type SingleFlight = <T>(factory: () => Promise<T>) => Promise<T>;

export function createSingleFlight(): SingleFlight {
  let current: Promise<unknown> | null = null;

  return <T>(factory: () => Promise<T>) => {
    if (current) return current as Promise<T>;

    const request = factory();
    current = request;
    const clear = () => {
      if (current === request) current = null;
    };
    void request.then(clear, clear);
    return request;
  };
}
