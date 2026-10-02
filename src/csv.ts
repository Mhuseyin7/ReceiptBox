/** Creates a CSV cell while neutralizing spreadsheet formula prefixes. */
export const safeCsvCell = (value:string) => '"'+value.replaceAll('"','""').replace(/^[-+=@]/,"'$&")+'"'
