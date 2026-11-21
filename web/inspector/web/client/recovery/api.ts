/** Local simulation adapter for the alternate recovery experience. */
import { recoveryRequest } from '../../../host/provider';
export class ApiFailure extends Error { constructor(message:string,readonly code:string,readonly uncertain:boolean){super(message);} }
export async function request<T>(url:string,options?:{body:unknown;csrf:string}):Promise<T> {
  try {return await recoveryRequest(url,options?.body) as T;} catch(error) {throw error instanceof ApiFailure?error:new ApiFailure(error instanceof Error?error.message:'模拟操作失败','mock_rejected',false);}
}
