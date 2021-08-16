import type { IUploadTask } from './types';
import { setProgress } from '../utils/store';

export async function get<T>(url: string): Promise<T> {
  let response: Response;
  try {
    response = await fetch(url);
  } catch (e) {
    throw e;
  }

  if (!response.ok) {
    throw new Error(response.status.toString());
  }

  return await response.json();
}

export async function post<T, S>(url: string, payload: T, jsonResponse: boolean): Promise<S> {
  let response: Response;
  try {
    response = await fetch(url, { body: JSON.stringify(payload), method: 'POST' });

  } catch (e) {
    throw e;
  }

  if (!response.ok) {
    throw new Error(response.status.toString());
  }

  return jsonResponse ? await response.json() : await response.text();
}

// TODO: process uploading.
export async function upload(task: IUploadTask) {
  const file = task.file;
  const filesize = file.size;
  const buffer = await file.arrayBuffer();
  const worker = new Worker('upload.js');
  const length = 10 * 1024 * 1024;

  const payload = {
    filename: file.name,
    size: filesize,
  };

  let uploadId: string = await post("/api/file/before-upload", payload, false);

  let start = 0;
  let transferredBytes = 0;
  let end = Math.min(start + length, filesize);
  let slice = buffer.slice(start, end);
  worker.postMessage({ type: "uploadId", data: uploadId });
  worker.postMessage({ type: "data", data: slice });

  worker.onmessage = async (e) => {
    const message = e.data;
    if (message.type === "progress") {
      transferredBytes += message.data;
    } else if (message.type === "done") {
      transferredBytes = end;

      if (end < filesize) {
        start = end;
        end = Math.min(start + length, filesize);
        slice = buffer.slice(start, end);
        worker.postMessage({ type: "data", data: slice });
      } else {
        worker.terminate();
        const payload = {
          upload_id: uploadId,
        };
        await post(`/api/file/finish-upload`, payload, false);
      }
    } else if (message.type === "error") {
      // TODO: retry several times.
    }

    task.progress = transferredBytes / filesize;
    setProgress(task.id, task.progress);
  };
}
