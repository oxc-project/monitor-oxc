export   type   Deep<T> = { [K in keyof T] : T[K] extends object ? Deep<T[K]> : T[K] }
export function pick<T,K extends keyof T>(obj:T, ...keys:K[]):Pick<T,K>{ const out={} as Pick<T,K>;for(const k of keys){out[k]=obj[k]}return out}
const   greeting=`hello ${ "world" }`;export default greeting
