import { useCallback, useEffect, useState } from 'react';

interface UsePersistedStateOptions<T> {
  deserialize?: (value: string) => Partial<T>;
  serialize?: (value: T) => string;
}

export function usePersistedState<T extends object>(
  key: string,
  initialValue: T,
  options: UsePersistedStateOptions<T> = {}
): [T, (action: Partial<T> | ((prevState: T) => Partial<T>)) => void] {
  const { deserialize = JSON.parse, serialize = JSON.stringify } = options;

  const [state, setFullState] = useState<T>(() => {
    if (typeof window === 'undefined') {
      return initialValue;
    }

    const savedValue = window.localStorage.getItem(key);

    if (savedValue === null) {
      return initialValue;
    }

    try {
      return {
        ...initialValue,
        ...deserialize(savedValue),
      };
    } catch (error) {
      console.warn(`Error reading ${key} from localStorage:`, error);
      return initialValue;
    }
  });

  useEffect(() => {
    if (typeof window === 'undefined') {
      return;
    }

    try {
      window.localStorage.setItem(key, serialize(state));
    } catch (error) {
      console.warn(`Error saving ${key} to localStorage:`, error);
    }
  }, [key, serialize, state]);

  const setState = useCallback(
    (action: Partial<T> | ((prevState: T) => Partial<T>)) => {
      setFullState((prevState) => ({
        ...prevState,
        ...(typeof action === 'function' ? action(prevState) : action),
      }));
    },
    []
  );

  return [state, setState];
}
