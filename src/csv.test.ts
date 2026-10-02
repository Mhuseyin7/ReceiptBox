import { describe, expect, it } from 'vitest'
import { safeCsvCell } from './csv'

describe('safeCsvCell', () => {
  it('escapes quotes and prevents spreadsheet formula execution', () => {
    expect(safeCsvCell('=SUM(A1:A2)')).toBe("\"'=SUM(A1:A2)\"")
    expect(safeCsvCell('Mert "Receipt"')).toBe('"Mert ""Receipt"""')
  })
})
