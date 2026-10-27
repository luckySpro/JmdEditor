import { describe, expect, it } from 'vitest'
import { remarkMergeHtml, rendersEmpty } from './html'

describe('remarkMergeHtml', () => {
  it('joins a paragraph made only of tags into one html node', () => {
    const tree = {
      type: 'root',
      children: [
        {
          type: 'paragraph',
          children: [
            { type: 'html', value: '<a href="x">' },
            { type: 'html', value: '<img src="a.png">' },
            { type: 'html', value: '</a>' },
            { type: 'text', value: ' ' },
            { type: 'html', value: '<img src="b.png">' },
          ],
        },
      ],
    }
    remarkMergeHtml()(tree)
    expect(tree.children[0].children).toEqual([{ type: 'html', value: '<a href="x"><img src="a.png"></a> <img src="b.png">' }])
  })

  it('leaves tags mixed with words alone', () => {
    const para = {
      type: 'paragraph',
      children: [
        { type: 'html', value: '<kbd>' },
        { type: 'text', value: 'Ctrl' },
        { type: 'html', value: '</kbd>' },
      ],
    }
    remarkMergeHtml()({ type: 'root', children: [para] })
    expect(para.children).toHaveLength(3)
  })
})

describe('rendersEmpty', () => {
  it('spots wrapper tags that show nothing', () => {
    expect(rendersEmpty('<div align="center"></div>')).toBe(true)
    expect(rendersEmpty('<img src="a.png">')).toBe(false)
    expect(rendersEmpty('<b>hi</b>')).toBe(false)
  })
})
