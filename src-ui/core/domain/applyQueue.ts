// 所有槽位与整组应用共用提交顺序；失败只影响当前操作，后续点击仍会执行。
export function orderedQueue() {
  let tail: Promise<unknown> = Promise.resolve();
  return <T>(action: () => Promise<T>): Promise<T> => {
    const result = tail.then(action);
    tail = result.catch(() => undefined);
    return result;
  };
}
